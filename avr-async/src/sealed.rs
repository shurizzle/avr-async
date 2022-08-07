#![allow(dead_code, path_statements, clippy::no_effect)]

pub(crate) const fn greater_than<const L: usize, const R: usize>() {
    Assert::<L, R>::GREATER;
}

pub(crate) const fn greater_than_eq<const L: usize, const R: usize>() {
    Assert::<L, R>::GREATER_EQ;
}

pub(crate) const fn less_than<const L: usize, const R: usize>() {
    Assert::<L, R>::LESS;
}

pub(crate) const fn less_than_eq<const L: usize, const R: usize>() {
    Assert::<L, R>::LESS_EQ;
}

pub(crate) const fn eq<const L: usize, const R: usize>() {
    Assert::<L, R>::EQ;
}

pub(crate) const fn not_eq<const L: usize, const R: usize>() {
    Assert::<L, R>::NOT_EQ;
}

pub(crate) const fn power_of_two<const N: usize>() {
    Assert::<N, 0>::GREATER;
    Assert::<N, 0>::POWER_OF_TWO;
}

pub(crate) const fn greater_than_0<const N: usize>() {
    greater_than::<N, 0>();
}

pub(crate) const fn greater_than_1<const N: usize>() {
    greater_than::<N, 0>();
}

pub(crate) const fn greater_than_eq_0<const N: usize>() {
    greater_than_eq::<N, 0>();
}

pub(crate) const fn greater_than_eq_1<const N: usize>() {
    greater_than_eq::<N, 1>();
}

struct Assert<const L: usize, const R: usize>;

impl<const L: usize, const R: usize> Assert<L, R> {
    pub const GREATER_EQ: usize = L - R;

    pub const LESS_EQ: usize = R - L;

    #[allow(clippy::erasing_op)]
    pub const NOT_EQ: isize = 0 / (R as isize - L as isize);

    pub const EQ: usize = (R - L) + (L - R);

    pub const GREATER: usize = L - R - 1;

    pub const LESS: usize = R - L - 1;

    pub const POWER_OF_TWO: usize = 0 - (L & (L - 1));
}
