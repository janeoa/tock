/* Memory Space Definitions, 1M flash, 512K ram */
MEMORY
{
  rom (rx)  : ORIGIN = 0x00000000, LENGTH = 256K
  prog (rx) : ORIGIN = 0x00040000, LENGTH = 768K
  #rom (rx)  : ORIGIN = 0x00000000, LENGTH = 768K
  #prog (rx) : ORIGIN = 0x000C0000, LENGTH = 256K
  ram (rwx) : ORIGIN = 0x20000000, LENGTH = 512K
}

PAGE_SIZE = 4K;
