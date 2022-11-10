use core::{future::Future, pin::Pin, task::Poll};

enum State<'a, 'b> {
    Start(super::super::raw::Start<'a>, u8, &'b [u8]),
    SlaW(super::super::raw::SlaRw<'a>, &'b [u8]),
    Write(super::super::raw::Write<'a, 'b>),
}

pub struct Write<'a, 'b, 'c, TWI: super::super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> {
    tx: &'a mut super::Transaction<'b, TWI, SDA, SCL, CLOCK>,
    state: Option<State<'b, 'c>>,
}

impl<'a, 'b, 'c, TWI: super::super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK>
    Write<'a, 'b, 'c, TWI, SDA, SCL, CLOCK>
{
    pub(crate) fn new(
        tx: &'a mut super::Transaction<'b, TWI, SDA, SCL, CLOCK>,
        addr: u8,
        buf: &'c [u8],
    ) -> Self {
        let state = Some({
            let twi = unsafe {
                &mut *(tx.twi as *mut super::super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK>)
            };
            let start = twi.start();
            State::Start(start, addr, buf)
        });

        Self { tx, state }
    }

    fn twi_ptr(&mut self) -> *mut super::super::raw::TwoWireInterface<TWI, SDA, SCL, CLOCK> {
        self.tx.twi as *mut _
    }
}

impl<'a, 'b, 'c, TWI: super::super::peripheral::TwiOps<SDA, SCL>, SDA, SCL, CLOCK> Future
    for Write<'a, 'b, 'c, TWI, SDA, SCL, CLOCK>
{
    type Output = Result<(), super::super::Error>;

    fn poll(self: Pin<&mut Self>, cx: &mut core::task::Context<'_>) -> Poll<Self::Output> {
        let this = Pin::get_mut(self);

        loop {
            match this.state.take().unwrap() {
                State::Start(mut fut, addr, buf) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.state.replace(State::Start(fut, addr, buf));
                        break Poll::Pending;
                    }
                    Poll::Ready(res) => match res {
                        Ok(()) => {
                            this.tx.started = true;
                            let slaw = unsafe { &mut *(this.twi_ptr()) }
                                .sla_rw(addr, super::super::peripheral::Direction::Write);
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
                        Err(err) => break Poll::Ready(Err(err)),
                    },
                },
                State::Write(mut fut) => match Pin::new(&mut fut).poll(cx) {
                    Poll::Pending => {
                        this.state.replace(State::Write(fut));
                        break Poll::Pending;
                    }
                    Poll::Ready(res) => break Poll::Ready(res),
                },
            }
        }
    }
}
