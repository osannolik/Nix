#include "soft_spi.h"

void soft_spi_init(spi_buffer_t *sspi, uint8_t cs_pin, uint8_t mosi_pin, uint8_t clk_pin) {
  nrf_mtx_init(&sspi->mutex);
  sspi->updated = false;
  sspi->cs_pin = cs_pin;
  sspi->mosi_pin = mosi_pin;
  sspi->clk_pin = clk_pin;
  sspi->delay_us = 100;

  nrf_gpio_cfg_output(cs_pin);
  nrf_gpio_pin_set(cs_pin);

  nrf_gpio_cfg_output(mosi_pin);
  nrf_gpio_pin_clear(mosi_pin);

  nrf_gpio_cfg_output(clk_pin);
  nrf_gpio_pin_clear(clk_pin);
}

bool soft_spi_put_data(spi_buffer_t *sspi, uint8_t *data) {
  const bool got_it = nrf_mtx_trylock(&sspi->mutex);

  if (got_it) {
    sspi->buffer[0] = CMD_ID_VOLTAGE_TEMPERATURE_MEASUREMENT;
    memcpy(&sspi->buffer[1], data, SPI_BUFFER_SIZE - 1);

    sspi->updated = true;

    nrf_mtx_unlock(&sspi->mutex);
  }

  return got_it;
}

bool soft_spi_write_buffer(spi_buffer_t *sspi) {
  const bool got_it = nrf_mtx_trylock(&sspi->mutex);
  bool did_it = false;

  if (got_it) {
    if (sspi->updated) {
      nrf_gpio_pin_clear(sspi->cs_pin);

      for (uint8_t byte_nr = 0; byte_nr < SPI_BUFFER_SIZE; byte_nr++) {
        for (uint8_t bit_mask = 0x80; bit_mask > 0; bit_mask >>= 1) {
          nrf_delay_us(sspi->delay_us);
          nrf_gpio_pin_clear(sspi->clk_pin);

          if (sspi->buffer[byte_nr] & bit_mask) {
            nrf_gpio_pin_set(sspi->mosi_pin);
          } else {
            nrf_gpio_pin_clear(sspi->mosi_pin);
          }

          nrf_delay_us(sspi->delay_us);
          nrf_gpio_pin_set(sspi->clk_pin);
        }
      }

      nrf_delay_us(sspi->delay_us);

      nrf_gpio_pin_set(sspi->cs_pin);
      nrf_gpio_pin_clear(sspi->mosi_pin);
      nrf_gpio_pin_clear(sspi->clk_pin);

      sspi->updated = false;
      did_it = true;
    }

    nrf_mtx_unlock(&sspi->mutex);
  }

  return did_it;
}
