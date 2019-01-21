//! Timers
use cast::{u16, u32};
use cortex_m::peripheral::syst::SystClkSource;
use cortex_m::peripheral::SYST;
use hal::timer::{CountDown, Periodic};
use nb;
use void::Void;

use crate::rcc::{Clocks, Rcc};
use crate::stm32::{TIM1, TIM14, TIM15, TIM16, TIM17, TIM2, TIM3, TIM6, TIM7};
use crate::time::MicroSecond;

pub trait TimerExt<TIM> {
    fn timer<T>(self, timeout: T, rcc: &mut Rcc) -> Timer<TIM>
    where
        T: Into<MicroSecond>;
}

/// Hardware timers
pub struct Timer<TIM> {
    clocks: Clocks,
    tim: TIM,
}

/// System timer
impl Timer<SYST> {
    /// Configures the SYST clock as a periodic count down timer
    pub fn syst<T>(mut syst: SYST, timeout: T, rcc: &mut Rcc) -> Self
    where
        T: Into<MicroSecond>,
    {
        syst.set_clock_source(SystClkSource::Core);
        let mut timer = Timer {
            tim: syst,
            clocks: rcc.clocks,
        };
        timer.start(timeout);
        timer
    }

    /// Starts listening
    pub fn listen(&mut self) {
        self.tim.enable_interrupt()
    }

    /// Stops listening
    pub fn unlisten(&mut self) {
        self.tim.disable_interrupt()
    }
}

impl CountDown for Timer<SYST> {
    type Time = MicroSecond;

    fn start<T>(&mut self, timeout: T)
    where
        T: Into<MicroSecond>,
    {
        let ticks = timeout.into().ticks(self.clocks.core_clk);
        assert!(ticks < (1 << 24));
        self.tim.set_reload(ticks);
        self.tim.clear_current();
        self.tim.enable_counter();
    }

    fn wait(&mut self) -> nb::Result<(), Void> {
        if self.tim.has_wrapped() {
            Ok(())
        } else {
            Err(nb::Error::WouldBlock)
        }
    }
}

impl TimerExt<SYST> for SYST {
    fn timer<T>(self, timeout: T, rcc: &mut Rcc) -> Timer<SYST>
    where
        T: Into<MicroSecond>,
    {
        Timer::syst(self, timeout, rcc)
    }
}

impl Periodic for Timer<SYST> {}

macro_rules! timers {
    ($($TIM:ident: ($tim:ident, $timXen:ident, $timXrst:ident, $apbenr:ident, $apbrstr:ident),)+) => {
        $(
            impl TimerExt<$TIM> for $TIM {
                fn timer<T>(self, timeout: T, rcc: &mut Rcc) -> Timer<$TIM>
                    where
                        T: Into<MicroSecond>,
                {
                    Timer::$tim(self, timeout, rcc)
                }
            }

            impl Timer<$TIM> {
                /// Configures a TIM peripheral as a periodic count down timer
                pub fn $tim<T>(tim: $TIM, timeout: T, rcc: &mut Rcc) -> Self
                where
                    T: Into<MicroSecond>,
                {
                    rcc.rb.$apbenr.modify(|_, w| w.$timXen().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().set_bit());
                    rcc.rb.$apbrstr.modify(|_, w| w.$timXrst().clear_bit());

                    let mut timer = Timer {
                        tim,
                        clocks: rcc.clocks,
                    };
                    timer.start(timeout);
                    timer
                }

                /// Starts listening
                pub fn listen(&mut self) {
                    self.tim.dier.write(|w| w.uie().set_bit());
                }

                /// Stops listening
                pub fn unlisten(&mut self) {
                    self.tim.dier.write(|w| w.uie().clear_bit());
                }

                /// Clears interrupt flag
                pub fn clear_irq(&mut self) {
                    self.tim.sr.write(|w| w.uif().clear_bit());
                }

                /// Releases the TIM peripheral
                pub fn release(self) -> $TIM {
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    self.tim
                }
            }

            impl CountDown for Timer<$TIM> {
                type Time = MicroSecond;

                fn start<T>(&mut self, timeout: T)
                where
                    T: Into<MicroSecond>,
                {
                    // pause
                    self.tim.cr1.modify(|_, w| w.cen().clear_bit());
                    // reset counter
                    self.tim.cnt.reset();

                    // Calculate counter config
                    let ticks = timeout.into().ticks(self.clocks.apb_tim_clk);
                    let psc = u16((ticks) / (1 << 16)).unwrap();
                    let arr = ticks / u32(psc + 1);

                    self.tim.psc.write(|w| unsafe { w.psc().bits(psc) });
                    self.tim.arr.write(|w| unsafe { w.bits(arr) });

                    self.tim.cr1.modify(|_, w| w.urs().set_bit());
                    self.tim.cr1.modify(|_, w| w.cen().set_bit());
                }

                fn wait(&mut self) -> nb::Result<(), Void> {
                    if self.tim.sr.read().uif().bit_is_clear() {
                        Err(nb::Error::WouldBlock)
                    } else {
                        self.tim.sr.modify(|_, w| w.uif().clear_bit());
                        Ok(())
                    }
                }
            }

            impl Periodic for Timer<$TIM> {}
        )+
    }
}

timers! {
    TIM1:  (tim1,  tim1en,  tim1rst,  apbenr2, apbrstr2),
    TIM2:  (tim2,  tim2en,  tim2rst,  apbenr1, apbrstr1),
    TIM3:  (tim3,  tim3en,  tim3rst,  apbenr1, apbrstr1),
    TIM6:  (tim6,  tim6en,  tim6rst,  apbenr1, apbrstr1),
    TIM7:  (tim7,  tim7en,  tim7rst,  apbenr1, apbrstr1),
    TIM14: (tim14, tim14en, tim14rst, apbenr2, apbrstr2),
    TIM15: (tim15, tim15en, tim15rst, apbenr2, apbrstr2),
    TIM16: (tim16, tim16en, tim16rst, apbenr2, apbrstr2),
    TIM17: (tim17, tim17en, tim17rst, apbenr2, apbrstr2),
}