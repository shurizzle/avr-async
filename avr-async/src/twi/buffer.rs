use core::slice::{Iter, IterMut};

pub trait OutputBuffer {
    fn next(&mut self) -> Option<u8>;
}

pub trait InputBuffer {
    fn push(&mut self, byte: u8);

    fn is_last(&mut self) -> bool;
}

pub struct SliceOutputBuffer<'a> {
    iter: Iter<'a, u8>,
}

impl<'a> SliceOutputBuffer<'a> {
    #[inline]
    pub fn new(slice: &'a [u8]) -> Self {
        Self { iter: slice.iter() }
    }

    #[inline]
    pub fn as_dyn(&mut self) -> &mut dyn OutputBuffer {
        self as &mut dyn OutputBuffer
    }
}

impl<'a> OutputBuffer for SliceOutputBuffer<'a> {
    #[inline]
    fn next(&mut self) -> Option<u8> {
        self.iter.next().cloned()
    }
}

pub trait IntoOutputBuffer {
    type Buffer: OutputBuffer;

    fn into_output_buffer(self) -> Self::Buffer;
}

impl<'a> IntoOutputBuffer for &'a [u8] {
    type Buffer = SliceOutputBuffer<'a>;

    #[inline]
    fn into_output_buffer(self) -> Self::Buffer {
        SliceOutputBuffer::new(self)
    }
}

pub struct SliceInputBuffer<'a> {
    iter: IterMut<'a, u8>,
}

impl<'a> SliceInputBuffer<'a> {
    #[inline]
    pub fn new(slice: &'a mut [u8]) -> Self {
        Self {
            iter: slice.iter_mut(),
        }
    }

    #[inline]
    pub fn as_dyn(&mut self) -> &mut dyn InputBuffer {
        self as &mut dyn InputBuffer
    }
}

impl<'a> InputBuffer for SliceInputBuffer<'a> {
    #[inline]
    fn push(&mut self, byte: u8) {
        if let Some(x) = self.iter.next() {
            *x = byte;
        }
    }

    #[inline]
    fn is_last(&mut self) -> bool {
        self.iter.len() == 1
    }
}

pub trait IntoInputBuffer {
    type Buffer: InputBuffer;

    fn into_input_buffer(self) -> Self::Buffer;
}

impl<'a> IntoInputBuffer for &'a mut [u8] {
    type Buffer = SliceInputBuffer<'a>;

    fn into_input_buffer(self) -> Self::Buffer {
        SliceInputBuffer::new(self)
    }
}
