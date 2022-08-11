#![allow(dead_code)]

pub trait Tuple {}

impl Tuple for () {}
impl<T> Tuple for (T,) {}
impl<T1, T2> Tuple for (T1, T2) {}
impl<T1, T2, T3> Tuple for (T1, T2, T3) {}
impl<T1, T2, T3, T4> Tuple for (T1, T2, T3, T4) {}
impl<T1, T2, T3, T4, T5> Tuple for (T1, T2, T3, T4, T5) {}
impl<T1, T2, T3, T4, T5, T6> Tuple for (T1, T2, T3, T4, T5, T6) {}
impl<T1, T2, T3, T4, T5, T6, T7> Tuple for (T1, T2, T3, T4, T5, T6, T7) {}
impl<T1, T2, T3, T4, T5, T6, T7, T8> Tuple for (T1, T2, T3, T4, T5, T6, T7, T8) {}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9> Tuple for (T1, T2, T3, T4, T5, T6, T7, T8, T9) {}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10> Tuple for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10) {}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11> Tuple
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11)
{
}
impl<T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12> Tuple
    for (T1, T2, T3, T4, T5, T6, T7, T8, T9, T10, T11, T12)
{
}
