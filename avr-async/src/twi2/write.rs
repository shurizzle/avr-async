use core::{future::Future, pin::Pin, task::Poll};

enum State<'a, 'b> {
    Wait(super::Wait<'a>, u8, &'b [u8]),
    Start(super::raw::Start<'a>, u8, &'b [u8]),
    SlaW(super::raw::SlaRw<'a>, &'b [u8]),
    Write(super::raw::Write<'a, 'b>),
    Stop(super::raw::Stop<'a>, Result<(), super::Error>),
}

pub struct Write<'a, 'b, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    twi: &'a mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>,
    state: Option<State<'a, 'b>>,
}

impl<'a, 'b, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>
    Write<'a, 'b, TWI, SDA, SCL, CLOCK>
{
    pub(crate) fn new(
        twi: &'a mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>,
        addr: u8,
        buf: &'b [u8],
    ) -> Self {
        let state = {
            let twi =
                unsafe { &mut *(twi as *mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>) };

            if twi.inner.set {
                State::Wait(super::Wait::new(twi), addr, buf)
            } else {
                State::Start(twi.start(), addr, buf)
            }
        };

        Self {
            twi,
            state: Some(state),
        }
    }

    fn twi_ptr(&mut self) -> *mut super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK> {
        self.twi as *mut _
    }
}

impl<'a, 'b, TWI: super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> Future
    for Write<'a, 'b, TWI, SDA, SCL, CLOCK>
{
    type Output = Result<(), super::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = Pin::get_mut(self);

        loop {
            match this.state.take().unwrap() {
                State::Wait(mut fut, addr, buf) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.state.replace(State::Wait(fut, addr, buf));
                        break Poll::Pending;
                    }
                    Poll::Ready(()) => {
                        let start = unsafe { &mut *(this.twi_ptr()) }.start();
                        this.state.replace(State::Start(start, addr, buf));
                    }
                },
                State::Start(mut fut, addr, buf) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.state.replace(State::Start(fut, addr, buf));
                        break Poll::Pending;
                    }
                    Poll::Ready(res) => match res {
                        Ok(()) => {
                            let slaw = unsafe { &mut *(this.twi_ptr()) }
                                .sla_rw(addr, super::peripheral::Direction::Write);
                            this.state.replace(State::SlaW(slaw, buf));
                        }
                        Err(err) => break Poll::Ready(Err(err)),
                    },
                },
                State::SlaW(mut fut, buf) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.state.replace(State::SlaW(fut, buf));
                        break Poll::Pending;
                    }
                    Poll::Ready(res) => match res {
                        Ok(()) => {
                            let write = unsafe { &mut *(this.twi_ptr()) }.write(buf);
                            this.state.replace(State::Write(write));
                        }
                        Err(err) => {
                            if err == super::Error::AddressNack {
                                crate::led2();
                            }
                            let stop = unsafe { &mut *(this.twi_ptr()) }.stop();
                            this.state.replace(State::Stop(stop, Err(err)));
                        }
                    },
                },
                State::Write(mut fut) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.state.replace(State::Write(fut));
                        break Poll::Pending;
                    }
                    Poll::Ready(res) => {
                        let stop = unsafe { &mut *(this.twi_ptr()) }.stop();
                        this.state.replace(State::Stop(stop, res));
                    }
                },
                State::Stop(mut fut, res) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.state.replace(State::Stop(fut, res));
                        break Poll::Pending;
                    }
                    Poll::Ready(()) => break Poll::Ready(res),
                },
            }
        }
    }
}
