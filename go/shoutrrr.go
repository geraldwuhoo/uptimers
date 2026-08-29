package main

/*
#include <stdlib.h>
*/
import "C"

import (
	"unsafe"

	"github.com/containrrr/shoutrrr"
)

// Shoutrrr sends msg to the given shoutrrr URL.
//
// It returns NULL on success. On failure it returns a C string describing the
// error, allocated with malloc and owned by the caller, which must be released
// with ShoutrrrFree.
//
//export Shoutrrr
func Shoutrrr(url *C.char, msg *C.char) *C.char {
	if err := shoutrrr.Send(C.GoString(url), C.GoString(msg)); err != nil {
		return C.CString(err.Error())
	}
	return nil
}

// ShoutrrrFree releases an error string returned by Shoutrrr. Passing NULL is a
// no-op, matching free(3).
//
//export ShoutrrrFree
func ShoutrrrFree(err *C.char) {
	C.free(unsafe.Pointer(err))
}

func main() {}
