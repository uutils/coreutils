#include <unistd.h>

const uid_t USER_UID = 1000;
const uid_t ROOT_UID = 0;

uid_t getuid(void) { return USER_UID; }
uid_t geteuid(void) { return ROOT_UID; }
