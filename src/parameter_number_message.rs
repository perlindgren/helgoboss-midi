use crate::{
    extract_high_7_bit_value_from_14_bit_value, extract_low_7_bit_value_from_14_bit_value, Channel,
    ShortMessageFactory, U14, U7,
};
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

/// A MIDI Parameter Number message, either registered (RPN) or non-registered (NRPN).
///
/// MIDI systems emit those by sending up to 4 short Control Change messages in a row. The
/// [`ParameterNumberMessageScanner`] can be used to extract such messages from a stream of
/// [`ShortMessage`]s.
///
/// # Example
///
/// ```
/// use helgoboss_midi::{
///     controller_numbers, Channel, ParameterNumberMessage, RawShortMessage, U14,
/// };
///
/// let msg =
///     ParameterNumberMessage::registered_14_bit(Channel::new(0), U14::new(420), U14::new(15000));
/// assert_eq!(msg.channel().get(), 0);
/// assert_eq!(msg.number().get(), 420);
/// assert_eq!(msg.value().get(), 15000);
/// assert!(msg.is_registered());
/// assert!(msg.is_14_bit());
/// let short_messages: [Option<RawShortMessage>; 4] = msg.to_short_messages();
/// use helgoboss_midi::test_util::control_change;
/// assert_eq!(
///     short_messages,
///     [
///         Some(control_change(0, 101, 3)),
///         Some(control_change(0, 100, 36)),
///         Some(control_change(0, 38, 24)),
///         Some(control_change(0, 6, 117)),
///     ]
/// );
/// ```
///
/// [`ShortMessage`]: trait.ShortMessage.html
/// [`ParameterNumberMessageScanner`]: struct.ParameterNumberMessageScanner.html
#[derive(Copy, Clone, Eq, PartialEq, Hash, Debug)]
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
pub struct ParameterNumberMessage {
    channel: Channel,
    number: U14,
    value: U14,
    is_registered: bool,
    is_14_bit: bool,
}

impl ParameterNumberMessage {
    /// Creates an NRPN message with a 7-bit value.
    pub fn non_registered_7_bit(
        channel: Channel,
        number: U14,
        value: U7,
    ) -> ParameterNumberMessage {
        Self::seven_bit(channel, number, value, false)
    }

    /// Creates an NRPN message with a 14-bit value.
    pub fn non_registered_14_bit(
        channel: Channel,
        number: U14,
        value: U14,
    ) -> ParameterNumberMessage {
        Self::fourteen_bit(channel, number, value, false)
    }

    /// Creates an RPN message with a 7-bit value.
    pub fn registered_7_bit(channel: Channel, number: U14, value: U7) -> ParameterNumberMessage {
        Self::seven_bit(channel, number, value, true)
    }

    /// Creates an RPN message with a 14-bit value.
    pub fn registered_14_bit(channel: Channel, number: U14, value: U14) -> ParameterNumberMessage {
        Self::fourteen_bit(channel, number, value, true)
    }

    fn seven_bit(
        channel: Channel,
        number: U14,
        value: U7,
        is_registered: bool,
    ) -> ParameterNumberMessage {
        ParameterNumberMessage {
            channel,
            number,
            value: value.into(),
            is_registered,
            is_14_bit: false,
        }
    }

    fn fourteen_bit(
        channel: Channel,
        number: U14,
        value: U14,
        is_registered: bool,
    ) -> ParameterNumberMessage {
        ParameterNumberMessage {
            channel,
            number,
            value,
            is_registered,
            is_14_bit: true,
        }
    }

    /// Returns the channel of this message.
    pub fn channel(&self) -> Channel {
        self.channel
    }

    /// Returns the parameter number of this message.
    pub fn number(&self) -> U14 {
        self.number
    }

    /// Returns the value of this message.
    ///
    /// If it's just a 7-bit message, the value is <= 127.
    pub fn value(&self) -> U14 {
        self.value
    }

    /// Returns `true` if this message has a 14-bit value and `false` if only a 7-bit value.
    pub fn is_14_bit(&self) -> bool {
        self.is_14_bit
    }

    /// Returns whether this message uses a registered parameter number.
    pub fn is_registered(&self) -> bool {
        self.is_registered
    }

    /// Translates this message into up to 4 short Control Change messages, which need to be sent in
    /// a row in order to encode this (N)RPN message.
    ///
    /// If this message has a 14-bit value, all returned messages are `Some`. If it has a 7-bit
    /// value only, the last one is `None`.
    pub fn to_short_messages<T: ShortMessageFactory>(&self) -> [Option<T>; 4] {
        use crate::controller_numbers::*;
        let mut messages = [None, None, None, None];
        let mut i = 0;
        // Number MSB
        messages[i] = Some(T::control_change(
            self.channel,
            if self.is_registered {
                REGISTERED_PARAMETER_NUMBER_MSB
            } else {
                NON_REGISTERED_PARAMETER_NUMBER_MSB
            },
            extract_high_7_bit_value_from_14_bit_value(self.number),
        ));
        i += 1;
        // Number LSB
        messages[i] = Some(T::control_change(
            self.channel,
            if self.is_registered {
                REGISTERED_PARAMETER_NUMBER_LSB
            } else {
                NON_REGISTERED_PARAMETER_NUMBER_LSB
            },
            extract_low_7_bit_value_from_14_bit_value(self.number),
        ));
        i += 1;
        // Value LSB
        if self.is_14_bit {
            messages[i] = Some(T::control_change(
                self.channel,
                DATA_ENTRY_MSB_LSB,
                extract_low_7_bit_value_from_14_bit_value(self.value),
            ));
            i += 1;
        }
        // Value MSB
        messages[i] = Some(T::control_change(
            self.channel,
            DATA_ENTRY_MSB,
            if self.is_14_bit {
                extract_high_7_bit_value_from_14_bit_value(self.value)
            } else {
                U7(self.value.get() as u8)
            },
        ));
        messages
    }
}

impl<T: ShortMessageFactory> From<ParameterNumberMessage> for [Option<T>; 4] {
    fn from(msg: ParameterNumberMessage) -> Self {
        msg.to_short_messages()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{channel as ch, controller_number as cn, u14, u7};
    use crate::RawShortMessage;

    #[test]
    fn parameter_number_messages_14_bit() {
        // Given
        let msg = ParameterNumberMessage::registered_14_bit(ch(0), u14(420), u14(15000));
        // When
        // Then
        assert_eq!(msg.channel(), ch(0));
        assert_eq!(msg.number(), u14(420));
        assert_eq!(msg.value(), u14(15000));
        assert!(msg.is_14_bit());
        assert!(msg.is_registered());
        let short_msgs: [Option<RawShortMessage>; 4] = msg.to_short_messages();
        assert_eq!(
            short_msgs,
            [
                Some(RawShortMessage::control_change(ch(0), cn(101), u7(3))),
                Some(RawShortMessage::control_change(ch(0), cn(100), u7(36))),
                Some(RawShortMessage::control_change(ch(0), cn(38), u7(24))),
                Some(RawShortMessage::control_change(ch(0), cn(6), u7(117))),
            ]
        );
    }

    #[test]
    #[should_panic]
    fn parameter_number_messages_7_bit_panic() {
        ParameterNumberMessage::non_registered_7_bit(ch(0), u14(420), u7(255));
    }

    #[test]
    fn parameter_number_messages_7_bit() {
        // Given
        let msg = ParameterNumberMessage::non_registered_7_bit(ch(2), u14(421), u7(126));
        // When
        // Then
        assert_eq!(msg.channel(), ch(2));
        assert_eq!(msg.number(), u14(421));
        assert_eq!(msg.value(), u14(126));
        assert!(!msg.is_14_bit());
        assert!(!msg.is_registered());
        let short_msgs: [Option<RawShortMessage>; 4] = msg.to_short_messages();
        assert_eq!(
            short_msgs,
            [
                Some(RawShortMessage::control_change(ch(2), cn(99), u7(3))),
                Some(RawShortMessage::control_change(ch(2), cn(98), u7(37))),
                Some(RawShortMessage::control_change(ch(2), cn(6), u7(126))),
                None,
            ]
        );
    }
}
