/* Smart Matter node — STM32G031J6 linker script.
 *
 * Matches the memory map in docs/whitepaper.md §2.4:
 *   0x0800_0000 – 0x0800_3FFF  Bootloader (16 KB)
 *   0x0800_4000 – 0x0800_BFFF  Application firmware (32 KB)  <-- this image
 *   0x0800_C000 – 0x0800_EEFF  Wear-level log store (12 KB)
 *   0x0800_F000 – 0x0800_F7FF  Configuration (2 KB)
 *   0x0800_F800 – 0x0800_FFFF  Free / OTA metadata (2 KB)
 */

MEMORY
{
  FLASH : ORIGIN = 0x08004000, LENGTH = 32K
  RAM   : ORIGIN = 0x20000000, LENGTH = 8K
}
