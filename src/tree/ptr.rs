use std::fmt;
use std::marker::PhantomData;

pub(crate) struct PackedPtr<I, L> {
    value: usize,
    inner: PhantomData<I>,
    leaf: PhantomData<L>,
}

pub(crate) enum Ptr<'a, I, L> {
    None,
    Inner(&'a I),
    Leaf(&'a L),
}

pub(crate) enum PtrMut<'a, I, L> {
    Inner(&'a mut I),
    Leaf(&'a mut L),
    None,
}

impl<I, L> PackedPtr<I, L> {
    pub(crate) fn null() -> PackedPtr<I, L> {
        PackedPtr {
            value: 0,
            inner: PhantomData,
            leaf: PhantomData,
        }
    }

    pub(crate) fn is_null(self) -> bool {
        self.value == 0
    }

    pub(crate) fn expand(&self) -> Ptr<'_, I, L> {
        if self.is_null() {
            Ptr::None
        } else if self.value & 0b1 == 0 {
            Ptr::Leaf(unsafe { &*(self.value as *const _) })
        } else {
            Ptr::Inner(unsafe { &*((self.value - 1) as *const _) })
        }
    }

    pub(crate) fn expand_mut(&mut self) -> PtrMut<'_, I, L> {
        if self.is_null() {
            PtrMut::None
        } else if self.value & 0b1 == 0 {
            PtrMut::Leaf(unsafe { &mut *(self.value as *mut _) })
        } else {
            PtrMut::Inner(unsafe { &mut *((self.value - 1) as *mut _) })
        }
    }

    pub(crate) fn from_inner(node: Box<I>) -> PackedPtr<I, L> {
        let value = Box::into_raw(node) as usize;
        debug_assert_eq!(value & 0b1, 0);
        PackedPtr {
            value: value | 1,
            leaf: PhantomData,
            inner: PhantomData,
        }
    }

    pub(crate) fn from_leaf(node: Box<L>) -> PackedPtr<I, L> {
        let value = Box::into_raw(node) as usize;
        debug_assert_eq!(value & 0b1, 0);
        PackedPtr {
            value: value,
            leaf: PhantomData,
            inner: PhantomData,
        }
    }
}

impl<I, L> Default for PackedPtr<I, L> {
    fn default() -> Self {
        PackedPtr::null()
    }
}

impl<I, L> Clone for PackedPtr<I, L> {
    fn clone(&self) -> Self {
        PackedPtr {
            value: self.value,
            leaf: PhantomData,
            inner: PhantomData,
        }
    }
}

impl<I, L> Copy for PackedPtr<I, L> {}

impl<I, L> PartialEq for PackedPtr<I, L> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<I, L> Eq for PackedPtr<I, L> {}

impl<I, L> fmt::Debug for PackedPtr<I, L> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_tuple("PackedPtr")
            .field(&format_args!("{:x?}", self.value))
            .finish()
    }
}
