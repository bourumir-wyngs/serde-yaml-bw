use std::marker::PhantomData;
use std::mem::{self, MaybeUninit};
use std::ops::Deref;
use std::ptr::{addr_of, NonNull};

pub(crate) struct Owned<T, Init = T> {
    ptr: NonNull<T>,
    marker: PhantomData<NonNull<Init>>,
}

impl<T> Owned<T> {
    pub fn new_uninit() -> Owned<MaybeUninit<T>, T> {
        let boxed = Box::<T>::new_uninit();
        Owned {
            // Box::into_raw never returns null; construct NonNull without unsafe.
            // Using NonNull::new + expect avoids unsafe while preserving the invariant.
            ptr: NonNull::new(Box::into_raw(boxed)).expect("Box::into_raw returned null"),
            marker: PhantomData,
        }
    }

    // SAFETY: `definitely_init` must contain a fully initialized `T`.
    pub unsafe fn assume_init(definitely_init: Owned<MaybeUninit<T>, T>) -> Owned<T> {
        let ptr = definitely_init.ptr;
        mem::forget(definitely_init);
        Owned {
            ptr: ptr.cast(),
            marker: PhantomData,
        }
    }
}

#[repr(transparent)]
pub(crate) struct InitPtr<T> {
    pub ptr: *mut T,
}

impl<T, Init> Deref for Owned<T, Init> {
    type Target = InitPtr<Init>;

    fn deref(&self) -> &Self::Target {
        // SAFETY: `self.ptr` is always valid and properly aligned; we only cast the
        // address to `InitPtr` for ergonomic field access.
        unsafe { &*addr_of!(self.ptr).cast::<InitPtr<Init>>() }
    }
}

impl<T, Init> Drop for Owned<T, Init> {
    fn drop(&mut self) {
        // SAFETY: `self.ptr` was allocated via `Box` and has unique ownership,
        // so reconstructing it here to drop is sound.
        let _ = unsafe { Box::from_raw(self.ptr.as_ptr()) };
    }
}
