#ifndef FAKE_STDIO_H
#define FAKE_STDIO_H

typedef __SIZE_TYPE__ size_t;
typedef __PTRDIFF_TYPE__ ptrdiff_t;
typedef __WCHAR_TYPE__ wchar_t;
#define NULL ((void*)0)

// Tcl needs these for function signatures
struct _iobuf;
typedef struct _iobuf FILE;

#endif
