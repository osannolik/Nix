#include <stdint.h>
#include <string.h>

#include "nrf_sdh.h"
#include "nrf_sdh_ble.h"
#include "nrf_pwr_mgmt.h"
#include "boards.h"
#include "ble.h"
#include "ble_advertising.h"
#include "nrf_ble_scan.h"

#include "nrf_delay.h"
#include "nrf_drv_spi.h"

#define SPI_SCK_PIN  12
#define SPI_MISO_PIN 14
#define SPI_MOSI_PIN 13
#define SPI_SS_PIN   29

#define SPI_INSTANCE  0
static const nrf_drv_spi_t spi = NRF_DRV_SPI_INSTANCE(SPI_INSTANCE);

#define SPI_BUFFER_SIZE 32
static uint8_t spi_buffer_tx[SPI_BUFFER_SIZE];
static uint8_t spi_buffer_rx[SPI_BUFFER_SIZE];

#define EXPECTED_NAME "sat"
#define MDATA_OFFSET (2)
#define MDATA_LENGTH (2 + 2)
#define CMD_ID_LENGTH (1)

enum spi_cmd_id_t {
  CMD_ID_VOLTAGE_TEMPERATURE_MEASUREMENT = 0x02,
};

#define CENTRAL_SCANNING_LED            BSP_BOARD_LED_0
#define CENTRAL_CONNECTED_LED           BSP_BOARD_LED_1
#define LEDBUTTON_LED                   BSP_BOARD_LED_2

#define APP_BLE_CONN_CFG_TAG            1
#define APP_BLE_OBSERVER_PRIO           3

NRF_BLE_SCAN_DEF(s_scan);

void assert_nrf_callback(uint16_t line_num, const uint8_t * p_file_name) {
  app_error_handler(0xDEADBEEF, line_num, p_file_name);
}

static void leds_init(void) {
  bsp_board_init(BSP_INIT_LEDS);

  bsp_board_leds_on();
}

static void scan_start(void) {
  ret_code_t err_code;

  err_code = nrf_ble_scan_start(&s_scan);
  APP_ERROR_CHECK(err_code);

//    bsp_board_led_off(CENTRAL_CONNECTED_LED);
//    bsp_board_led_on(CENTRAL_SCANNING_LED);
}

static void ble_evt_handler(ble_evt_t const * p_ble_evt, void * p_context) { }

static void ble_stack_init(void) {
  ret_code_t err_code;

  err_code = nrf_sdh_enable_request();
  APP_ERROR_CHECK(err_code);

  // Configure the BLE stack using the default settings.
  // Fetch the start address of the application RAM.
  uint32_t ram_start = 0;
  err_code = nrf_sdh_ble_default_cfg_set(APP_BLE_CONN_CFG_TAG, &ram_start);
  APP_ERROR_CHECK(err_code);

  // Enable BLE stack.
  err_code = nrf_sdh_ble_enable(&ram_start);
  APP_ERROR_CHECK(err_code);

  // Register a handler for BLE events.
  NRF_SDH_BLE_OBSERVER(m_ble_observer, APP_BLE_OBSERVER_PRIO, ble_evt_handler, NULL);
}

static void scan_evt_handler(scan_evt_t const * p_scan_evt) {
  const nrf_ble_scan_evt_filter_match_t match = p_scan_evt->params.filter_match;

  switch(p_scan_evt->scan_evt_id) {

    case NRF_BLE_SCAN_EVT_FILTER_MATCH:
      if (match.filter_match.name_filter_match) {
        uint8_t *mdata = ble_advdata_parse(match.p_adv_report->data.p_data,
                                           match.p_adv_report->data.len,
                                           BLE_GAP_AD_TYPE_MANUFACTURER_SPECIFIC_DATA);
        if (mdata) {
          spi_buffer_tx[0] = CMD_ID_VOLTAGE_TEMPERATURE_MEASUREMENT;
          memcpy(&spi_buffer_tx[1], &mdata[MDATA_OFFSET], MDATA_LENGTH);
          const size_t send_length = CMD_ID_LENGTH + MDATA_LENGTH;

          bsp_board_led_invert(CENTRAL_SCANNING_LED);

          APP_ERROR_CHECK(nrf_drv_spi_transfer(&spi, spi_buffer_tx, send_length, spi_buffer_rx, send_length));
        }
      }
      break;

    default:
      break;
  }
}

static void power_management_init(void) {
  ret_code_t err_code;
  err_code = nrf_pwr_mgmt_init();
  APP_ERROR_CHECK(err_code);
}

static void scan_init(const char* expected_name) {
  nrf_ble_scan_init_t init_scan;

  memset(&init_scan, 0, sizeof(init_scan));

  init_scan.connect_if_match = false;
  init_scan.conn_cfg_tag     = APP_BLE_CONN_CFG_TAG;

  ret_code_t err_code = nrf_ble_scan_init(&s_scan, &init_scan, scan_evt_handler);
  APP_ERROR_CHECK(err_code);

  err_code = nrf_ble_scan_filters_enable(&s_scan, NRF_BLE_SCAN_NAME_FILTER, false);
  APP_ERROR_CHECK(err_code);

  err_code = nrf_ble_scan_filter_set(&s_scan, SCAN_NAME_FILTER, expected_name);
  APP_ERROR_CHECK(err_code);
}

static void idle_state_handle(void) {
  nrf_pwr_mgmt_run();
}

void spi_event_handler(nrf_drv_spi_evt_t const * p_event, void *p_context) { }

static void spi_init(void) {
  nrf_drv_spi_config_t config = NRF_DRV_SPI_DEFAULT_CONFIG;
  config.ss_pin   = SPI_SS_PIN;
  config.miso_pin = SPI_MISO_PIN;
  config.mosi_pin = SPI_MOSI_PIN;
  config.sck_pin  = SPI_SCK_PIN;
  config.frequency = NRF_DRV_SPI_FREQ_125K;

  const ret_code_t err_code = nrf_drv_spi_init(&spi, &config, spi_event_handler, NULL);
  APP_ERROR_CHECK(err_code);
}

int main(void) {
  leds_init();
  power_management_init();
  spi_init();
  ble_stack_init();
  scan_init(EXPECTED_NAME);
  scan_start();

  //bsp_board_led_on(CENTRAL_SCANNING_LED);
  //bsp_board_led_on(CENTRAL_CONNECTED_LED);

  while(true) {
    idle_state_handle();
  }
}
