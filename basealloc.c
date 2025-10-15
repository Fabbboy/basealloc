#include <stddef.h>
#include <stdlib.h>

int main() {
  void *ptr = malloc(256);
  ptr = realloc(ptr, 512);
  free(ptr);
  return 0;
}