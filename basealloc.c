#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>

int main() {

  void *ptr = realloc((void *)0x123, 256);
  return 0;
}