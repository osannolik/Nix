#include <stdint.h>
#include <string.h>

#include "nrf_sdh.h"
#include "nrf_sdh_ble.h"
#include "nrf_pwr_mgmt.h"
#include "boards.h"
#include "ble.h"
#include "ble_advertising.h"
#include "nrf_ble_scan.h"

#define CENTRAL_SCANNING_LED            BSP_BOARD_LED_0
#define CENTRAL_CONNECTED_LED           BSP_BOARD_LED_1
#define LEDBUTTON_LED                   BSP_BOARD_LED_2

#define APP_BLE_CONN_CFG_TAG            1
#define APP_BLE_OBSERVER_PRIO           3

NRF_BLE_SCAN_DEF(s_scan);

static char const expected_peripheral_name[] = "sat";

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

static int dummy = 0;

static uint8_t temperature_bcd[2] = {0,0};
static uint8_t bat_voltage_bcd[2] = {0,0};

static void ble_evt_handler(ble_evt_t const * p_ble_evt, void * p_context) {
#if 0
  ble_gap_evt_t const * p_gap_evt = &p_ble_evt->evt.gap_evt;

    ble_gap_evt_adv_report_t adv_report = p_gap_evt->params.adv_report;

    switch (p_ble_evt->header.evt_id) {
        case BLE_GAP_EVT_ADV_REPORT:
            if (!adv_report.type.connectable && !adv_report.type.scannable && adv_report.data.len == 20) {

                memcpy(bat_voltage_bcd, &adv_report.data.p_data[11], 2);
                memcpy(temperature_bcd, &adv_report.data.p_data[13], 2);

                dummy++;
            }
            break;

        default:
            // No implementation needed.
            break;
    }
#endif
}

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
      if (match.filter_match.name_filter_match && (match.p_adv_report->data.len == 20)) {

        uint8_t *mdata = ble_advdata_parse(match.p_adv_report->data.p_data,
                                           match.p_adv_report->data.len,
                                           BLE_GAP_AD_TYPE_MANUFACTURER_SPECIFIC_DATA);

        if (mdata) {
          memcpy(bat_voltage_bcd, &mdata[2], 2);
          memcpy(temperature_bcd, &mdata[4], 2);

          bsp_board_led_invert(CENTRAL_SCANNING_LED);

          dummy--;
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

static void scan_init(void) {
  ret_code_t          err_code;
  nrf_ble_scan_init_t init_scan;

  memset(&init_scan, 0, sizeof(init_scan));

  init_scan.connect_if_match = false;
  init_scan.conn_cfg_tag     = APP_BLE_CONN_CFG_TAG;

  err_code = nrf_ble_scan_init(&s_scan, &init_scan, scan_evt_handler);
  APP_ERROR_CHECK(err_code);

  err_code = nrf_ble_scan_filters_enable(&s_scan, NRF_BLE_SCAN_NAME_FILTER, false);
  APP_ERROR_CHECK(err_code);

  err_code = nrf_ble_scan_filter_set(&s_scan, SCAN_NAME_FILTER, expected_peripheral_name);
  APP_ERROR_CHECK(err_code);
}

static void idle_state_handle(void) {
  nrf_pwr_mgmt_run();
}

int main(void) {
  leds_init();
  power_management_init();
  ble_stack_init();
  scan_init();
  scan_start();

  //bsp_board_led_on(CENTRAL_SCANNING_LED);
  //bsp_board_led_on(CENTRAL_CONNECTED_LED);

  while(true) {
    idle_state_handle();
  }
}
