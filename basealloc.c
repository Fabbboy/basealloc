#include <stddef.h>
#include <stdio.h>

extern size_t ba_page_size();

int main() {
  size_t page_size = ba_page_size();
  printf("Page size: %zu\n", page_size);
  return 0;
}