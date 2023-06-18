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
#include "app_timer.h"

#include "soft_spi.h"

#define SPI_SCK_PIN  12
#define SPI_MISO_PIN 14
#define SPI_MOSI_PIN 13
#define SPI_SS_PIN   29

#define EXPECTED_NAME "sat"

#define CENTRAL_SCANNING_LED            BSP_BOARD_LED_0
#define CENTRAL_CONNECTED_LED           BSP_BOARD_LED_1
#define LEDBUTTON_LED                   BSP_BOARD_LED_2

#define APP_BLE_CONN_CFG_TAG            1
#define APP_BLE_OBSERVER_PRIO           3

static spi_buffer_t soft_spi;

APP_TIMER_DEF(update_timer);

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
          soft_spi_put_data(&soft_spi, &mdata[MDATA_OFFSET]);
          bsp_board_led_invert(CENTRAL_SCANNING_LED);
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

void update(void * p_context) {
  (void) p_context;
  (void) soft_spi_write_buffer(&soft_spi);
}

int main(void) {
  leds_init();
  power_management_init();

  app_timer_init();
  app_timer_create(&update_timer, APP_TIMER_MODE_REPEATED, update);

  soft_spi_init(&soft_spi, SPI_SS_PIN, SPI_MOSI_PIN, SPI_SCK_PIN);
  ble_stack_init();
  scan_init(EXPECTED_NAME);
  scan_start();

  //bsp_board_led_on(CENTRAL_SCANNING_LED);
  //bsp_board_led_on(CENTRAL_CONNECTED_LED);

  int8_t err_code = app_timer_start(update_timer, APP_TIMER_TICKS(100), NULL);
  APP_ERROR_CHECK(err_code);

  while(true) {
    idle_state_handle();
  }
}
