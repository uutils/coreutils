#include "libstdbuf.h"

void __attribute ((constructor))
stdbuf_init (void)
{
	stdbuf();
}
