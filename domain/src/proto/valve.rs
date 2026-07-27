pub mod flow_controller_ {
    pub mod v1_ {
        /// Sent by the backend on each Class-A poll. The device executes
        /// `desired_state` and reports the result in the next Uplink. Backend sends
        /// OPEN or CLOSED; an unset (UNSPECIFIED) desired_state is ignored.
        #[derive(Debug, Default, PartialEq, Clone, Copy)]
        pub struct Downlink {
            pub r#desired_state: ValveState,
        }
        impl Downlink {
            /// Return a reference to `desired_state`
            #[inline]
            pub fn r#desired_state(&self) -> &ValveState {
                &self.r#desired_state
            }
            /// Return a mutable reference to `desired_state`
            #[inline]
            pub fn mut_desired_state(&mut self) -> &mut ValveState {
                &mut self.r#desired_state
            }
            /// Set the value of `desired_state`
            #[inline]
            pub fn set_desired_state(&mut self, value: ValveState) -> &mut Self {
                self.r#desired_state = value.into();
                self
            }
            /// Builder method that sets the value of `desired_state`. Useful for initializing the message.
            #[inline]
            pub fn init_desired_state(mut self, value: ValveState) -> Self {
                self.r#desired_state = value.into();
                self
            }
        }
        impl ::micropb::MessageDecode for Downlink {
            fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                &mut self,
                decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                len: usize,
            ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                let before = decoder.bytes_read();
                while decoder.bytes_read() - before < len {
                    let tag = decoder.decode_tag()?;
                    match tag.field_num() {
                        0 => return Err(::micropb::DecodeError::ZeroField),
                        1u32 => {
                            let mut_ref = &mut self.r#desired_state;
                            {
                                let val = decoder
                                    .decode_int32()
                                    .map(|n| ValveState(n as _))?;
                                let val_ref = &val;
                                if val_ref.0 != 0 {
                                    *mut_ref = val as _;
                                }
                            };
                        }
                        _ => {
                            decoder.skip_wire_value(tag.wire_type())?;
                        }
                    }
                }
                Ok(())
            }
        }
        impl ::micropb::MessageEncode for Downlink {
            const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                let mut max_size = 0;
                match ::micropb::const_map!(
                    ::core::result::Result::Ok(ValveState::_MAX_SIZE), | size | size +
                    1usize
                ) {
                    ::core::result::Result::Ok(size) => {
                        max_size += size;
                    }
                    ::core::result::Result::Err(err) => {
                        break 'msg (::core::result::Result::<usize, _>::Err(err));
                    }
                }
                ::core::result::Result::Ok(max_size)
            };
            fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                &self,
                encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
            ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                use ::micropb::{PbMap, FieldEncode};
                {
                    let val_ref = &self.r#desired_state;
                    if val_ref.0 != 0 {
                        encoder.encode_varint32(8u32)?;
                        encoder.encode_int32(val_ref.0 as _)?;
                    }
                }
                Ok(())
            }
            fn compute_size(&self) -> usize {
                use ::micropb::{PbMap, FieldEncode};
                let mut size = 0;
                {
                    let val_ref = &self.r#desired_state;
                    if val_ref.0 != 0 {
                        size += 1usize + ::micropb::size::sizeof_int32(val_ref.0 as _);
                    }
                }
                size
            }
        }
        /// Sent by the device on each scheduled uplink. `current_state` is what the
        /// device believes the physical valve is in right now;
        /// `last_commanded_state` is the most recent state the backend asked for.
        /// They diverge while a command is in flight or if actuation failed.
        #[derive(Debug, Default, PartialEq, Clone, Copy)]
        pub struct Uplink {
            pub r#current_state: ValveState,
            pub r#last_commanded_state: ValveState,
        }
        impl Uplink {
            /// Return a reference to `current_state`
            #[inline]
            pub fn r#current_state(&self) -> &ValveState {
                &self.r#current_state
            }
            /// Return a mutable reference to `current_state`
            #[inline]
            pub fn mut_current_state(&mut self) -> &mut ValveState {
                &mut self.r#current_state
            }
            /// Set the value of `current_state`
            #[inline]
            pub fn set_current_state(&mut self, value: ValveState) -> &mut Self {
                self.r#current_state = value.into();
                self
            }
            /// Builder method that sets the value of `current_state`. Useful for initializing the message.
            #[inline]
            pub fn init_current_state(mut self, value: ValveState) -> Self {
                self.r#current_state = value.into();
                self
            }
            /// Return a reference to `last_commanded_state`
            #[inline]
            pub fn r#last_commanded_state(&self) -> &ValveState {
                &self.r#last_commanded_state
            }
            /// Return a mutable reference to `last_commanded_state`
            #[inline]
            pub fn mut_last_commanded_state(&mut self) -> &mut ValveState {
                &mut self.r#last_commanded_state
            }
            /// Set the value of `last_commanded_state`
            #[inline]
            pub fn set_last_commanded_state(&mut self, value: ValveState) -> &mut Self {
                self.r#last_commanded_state = value.into();
                self
            }
            /// Builder method that sets the value of `last_commanded_state`. Useful for initializing the message.
            #[inline]
            pub fn init_last_commanded_state(mut self, value: ValveState) -> Self {
                self.r#last_commanded_state = value.into();
                self
            }
        }
        impl ::micropb::MessageDecode for Uplink {
            fn decode<IMPL_MICROPB_READ: ::micropb::PbRead>(
                &mut self,
                decoder: &mut ::micropb::PbDecoder<IMPL_MICROPB_READ>,
                len: usize,
            ) -> Result<(), ::micropb::DecodeError<IMPL_MICROPB_READ::Error>> {
                use ::micropb::{PbBytes, PbString, PbVec, PbMap, FieldDecode};
                let before = decoder.bytes_read();
                while decoder.bytes_read() - before < len {
                    let tag = decoder.decode_tag()?;
                    match tag.field_num() {
                        0 => return Err(::micropb::DecodeError::ZeroField),
                        1u32 => {
                            let mut_ref = &mut self.r#current_state;
                            {
                                let val = decoder
                                    .decode_int32()
                                    .map(|n| ValveState(n as _))?;
                                let val_ref = &val;
                                if val_ref.0 != 0 {
                                    *mut_ref = val as _;
                                }
                            };
                        }
                        2u32 => {
                            let mut_ref = &mut self.r#last_commanded_state;
                            {
                                let val = decoder
                                    .decode_int32()
                                    .map(|n| ValveState(n as _))?;
                                let val_ref = &val;
                                if val_ref.0 != 0 {
                                    *mut_ref = val as _;
                                }
                            };
                        }
                        _ => {
                            decoder.skip_wire_value(tag.wire_type())?;
                        }
                    }
                }
                Ok(())
            }
        }
        impl ::micropb::MessageEncode for Uplink {
            const MAX_SIZE: ::core::result::Result<usize, &'static str> = 'msg: {
                let mut max_size = 0;
                match ::micropb::const_map!(
                    ::core::result::Result::Ok(ValveState::_MAX_SIZE), | size | size +
                    1usize
                ) {
                    ::core::result::Result::Ok(size) => {
                        max_size += size;
                    }
                    ::core::result::Result::Err(err) => {
                        break 'msg (::core::result::Result::<usize, _>::Err(err));
                    }
                }
                match ::micropb::const_map!(
                    ::core::result::Result::Ok(ValveState::_MAX_SIZE), | size | size +
                    1usize
                ) {
                    ::core::result::Result::Ok(size) => {
                        max_size += size;
                    }
                    ::core::result::Result::Err(err) => {
                        break 'msg (::core::result::Result::<usize, _>::Err(err));
                    }
                }
                ::core::result::Result::Ok(max_size)
            };
            fn encode<IMPL_MICROPB_WRITE: ::micropb::PbWrite>(
                &self,
                encoder: &mut ::micropb::PbEncoder<IMPL_MICROPB_WRITE>,
            ) -> Result<(), IMPL_MICROPB_WRITE::Error> {
                use ::micropb::{PbMap, FieldEncode};
                {
                    let val_ref = &self.r#current_state;
                    if val_ref.0 != 0 {
                        encoder.encode_varint32(8u32)?;
                        encoder.encode_int32(val_ref.0 as _)?;
                    }
                }
                {
                    let val_ref = &self.r#last_commanded_state;
                    if val_ref.0 != 0 {
                        encoder.encode_varint32(16u32)?;
                        encoder.encode_int32(val_ref.0 as _)?;
                    }
                }
                Ok(())
            }
            fn compute_size(&self) -> usize {
                use ::micropb::{PbMap, FieldEncode};
                let mut size = 0;
                {
                    let val_ref = &self.r#current_state;
                    if val_ref.0 != 0 {
                        size += 1usize + ::micropb::size::sizeof_int32(val_ref.0 as _);
                    }
                }
                {
                    let val_ref = &self.r#last_commanded_state;
                    if val_ref.0 != 0 {
                        size += 1usize + ::micropb::size::sizeof_int32(val_ref.0 as _);
                    }
                }
                size
            }
        }
        /// State of (or desired state of) the irrigation valve.
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        #[repr(transparent)]
        pub struct ValveState(pub i32);
        impl ValveState {
            /// Maximum encoded size of the enum
            pub const _MAX_SIZE: usize = 10usize;
            /// Field unset / no known state.
            pub const Unspecified: Self = Self(0);
            pub const Open: Self = Self(1);
            pub const Closed: Self = Self(2);
        }
        impl core::default::Default for ValveState {
            fn default() -> Self {
                Self(0)
            }
        }
        impl core::convert::From<i32> for ValveState {
            fn from(val: i32) -> Self {
                Self(val)
            }
        }
    }
}
