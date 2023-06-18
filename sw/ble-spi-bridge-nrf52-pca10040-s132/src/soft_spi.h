/**
 * Need to use SPI with clock slower than 125 kHz, so just do it in software...
 */

#ifndef BLE_SPI_BRIDGE_NRF52_PCA10040_S132_SRC_SOFT_SPI_H_
#define BLE_SPI_BRIDGE_NRF52_PCA10040_S132_SRC_SOFT_SPI_H_

#include "nrf_delay.h"
#include "nrf_mtx.h"
#include "nrf_gpio.h"

#define MDATA_OFFSET (2)
#define MDATA_LENGTH (2 + 2)
#define CMD_ID_LENGTH (1)

enum spi_cmd_id_t {
  CMD_ID_VOLTAGE_TEMPERATURE_MEASUREMENT = 0x02,
};

#define SPI_BUFFER_SIZE (CMD_ID_LENGTH + MDATA_LENGTH)

typedef struct {
  nrf_mtx_t mutex;
  bool updated;
  uint8_t buffer[SPI_BUFFER_SIZE];
  uint8_t cs_pin;
  uint8_t mosi_pin;
  uint8_t clk_pin;
  uint32_t delay_us;
} spi_buffer_t;

void soft_spi_init(spi_buffer_t *sspi, uint8_t cs_pin, uint8_t mosi_pin, uint8_t clk_pin);

bool soft_spi_put_data(spi_buffer_t *sspi, uint8_t *data);

bool soft_spi_write_buffer(spi_buffer_t *sspi);

#endif //BLE_SPI_BRIDGE_NRF52_PCA10040_S132_SRC_SOFT_SPI_H_
