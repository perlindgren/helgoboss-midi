use crate::{
    build_14_bit_value_from_two_7_bit_values, Channel, ParameterNumberMessage, ShortMessage,
    StructuredShortMessage, U7,
};

/// Scanner for detecting (N)RPN messages in a stream of short messages.
///
/// # Example
///
/// ```
/// use helgoboss_midi::test_util::control_change;
/// use helgoboss_midi::{
///     Channel, ControllerNumber, ParameterNumberMessage, ParameterNumberMessageScanner, U14,
/// };
///
/// let mut scanner = ParameterNumberMessageScanner::new();
/// let result_1 = scanner.feed(&control_change(0, 101, 3));
/// let result_2 = scanner.feed(&control_change(0, 100, 36));
/// let result_3 = scanner.feed(&control_change(0, 38, 24));
/// let result_4 = scanner.feed(&control_change(0, 6, 117));
/// assert_eq!(result_1, None);
/// assert_eq!(result_2, None);
/// assert_eq!(result_3, None);
/// assert_eq!(
///     result_4,
///     Some(ParameterNumberMessage::registered_14_bit(
///         Channel::new(0),
///         U14::new(420),
///         U14::new(15000)
///     ))
/// );
/// ```
#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
pub struct ParameterNumberMessageScanner {
    scanner_by_channel: [ScannerForOneChannel; 16],
}

impl ParameterNumberMessageScanner {
    /// Creates a new scanner.
    pub fn new() -> ParameterNumberMessageScanner {
        Default::default()
    }

    /// Feeds the scanner a single short message.
    ///
    /// Returns the (N)RPN message if one has been detected.
    pub fn feed(&mut self, msg: &impl ShortMessage) -> Option<ParameterNumberMessage> {
        let channel = msg.channel()?;
        self.scanner_by_channel[usize::from(channel)].feed(msg)
    }

    /// Resets the scanner discarding all intermediate scanning progress.
    pub fn reset(&mut self) {
        for p in self.scanner_by_channel.iter_mut() {
            p.reset();
        }
    }
}

#[derive(Copy, Clone, Eq, PartialEq, Debug, Default)]
struct ScannerForOneChannel {
    number_msb: Option<U7>,
    number_lsb: Option<U7>,
    is_registered: bool,
    value_lsb: Option<U7>,
}

impl ScannerForOneChannel {
    fn feed(&mut self, msg: &impl ShortMessage) -> Option<ParameterNumberMessage> {
        match msg.to_structured() {
            StructuredShortMessage::ControlChange {
                channel,
                controller_number,
                control_value,
            } => match controller_number.get() {
                98 => self.process_number_lsb(control_value, false),
                99 => self.process_number_msb(control_value, false),
                100 => self.process_number_lsb(control_value, true),
                101 => self.process_number_msb(control_value, true),
                38 => self.process_value_lsb(control_value),
                6 => self.process_value_msb(channel, control_value),
                _ => None,
            },
            _ => None,
        }
    }

    fn reset(&mut self) {
        self.number_msb = None;
        self.number_lsb = None;
        self.is_registered = false;
        self.reset_value();
    }

    fn process_number_lsb(
        &mut self,
        number_lsb: U7,
        is_registered: bool,
    ) -> Option<ParameterNumberMessage> {
        self.reset_value();
        self.number_lsb = Some(number_lsb);
        self.is_registered = is_registered;
        None
    }

    fn process_number_msb(
        &mut self,
        number_msb: U7,
        is_registered: bool,
    ) -> Option<ParameterNumberMessage> {
        self.reset_value();
        self.number_msb = Some(number_msb);
        self.is_registered = is_registered;
        None
    }

    fn process_value_lsb(&mut self, value_lsb: U7) -> Option<ParameterNumberMessage> {
        self.value_lsb = Some(value_lsb);
        None
    }

    fn process_value_msb(
        &mut self,
        channel: Channel,
        value_msb: U7,
    ) -> Option<ParameterNumberMessage> {
        let number_lsb = self.number_lsb?;
        let number_msb = self.number_msb?;
        let number = build_14_bit_value_from_two_7_bit_values(number_msb, number_lsb);
        let msg = if self.is_registered {
            match self.value_lsb {
                Some(value_lsb) => ParameterNumberMessage::registered_14_bit(
                    channel,
                    number,
                    build_14_bit_value_from_two_7_bit_values(value_msb, value_lsb),
                ),
                None => ParameterNumberMessage::registered_7_bit(channel, number, value_msb),
            }
        } else {
            match self.value_lsb {
                Some(value_lsb) => ParameterNumberMessage::non_registered_14_bit(
                    channel,
                    number,
                    build_14_bit_value_from_two_7_bit_values(value_msb, value_lsb),
                ),
                None => ParameterNumberMessage::non_registered_7_bit(channel, number, value_msb),
            }
        };
        Some(msg)
    }

    fn reset_value(&mut self) {
        self.value_lsb = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_util::{channel as ch, controller_number as cn, key_number, u14, u7};
    use crate::{RawShortMessage, ShortMessageFactory};

    #[test]
    fn should_ignore_non_contributing_short_messages() {
        // Given
        let mut scanner = ParameterNumberMessageScanner::new();
        // When
        // Then
        assert_eq!(
            scanner.feed(&RawShortMessage::note_on(ch(0), key_number(100), u7(100))),
            None
        );
        assert_eq!(
            scanner.feed(&RawShortMessage::note_on(ch(0), key_number(100), u7(120))),
            None
        );
        assert_eq!(
            scanner.feed(&RawShortMessage::control_change(ch(0), cn(80), u7(1))),
            None
        );
    }

    #[test]
    fn should_return_parameter_number_result_message_on_fourth_short_message() {
        // Given
        let mut scanner = ParameterNumberMessageScanner::new();
        // When
        let result_1 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(101), u7(3)));
        let result_2 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(100), u7(36)));
        let result_3 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(38), u7(24)));
        let result_4 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(6), u7(117)));
        // Then
        assert_eq!(result_1, None);
        assert_eq!(result_2, None);
        assert_eq!(result_3, None);
        let result_4 = result_4.unwrap();
        assert_eq!(result_4.channel(), ch(0));
        assert_eq!(result_4.number(), u14(420));
        assert_eq!(result_4.value(), u14(15000));
        assert!(result_4.is_registered());
        assert!(result_4.is_14_bit());
    }

    #[test]
    fn should_return_parameter_number_result_message_on_third_short_message() {
        // Given
        let mut scanner = ParameterNumberMessageScanner::new();
        // When
        let result_1 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(99), u7(3)));
        let result_2 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(98), u7(37)));
        let result_3 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(6), u7(126)));
        // Then
        assert_eq!(result_1, None);
        assert_eq!(result_2, None);
        let result_3 = result_3.unwrap();
        assert_eq!(result_3.channel(), ch(2));
        assert_eq!(result_3.number(), u14(421));
        assert_eq!(result_3.value(), u14(126));
        assert!(!result_3.is_registered());
        assert!(!result_3.is_14_bit());
    }

    #[test]
    fn should_process_different_channels_independently() {
        // Given
        let mut scanner = ParameterNumberMessageScanner::new();
        // When
        let result_1 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(101), u7(3)));
        let result_2 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(99), u7(3)));
        let result_3 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(100), u7(36)));
        let result_4 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(98), u7(37)));
        let result_5 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(38), u7(24)));
        let result_6 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(6), u7(126)));
        let result_7 = scanner.feed(&RawShortMessage::control_change(ch(0), cn(6), u7(117)));
        // Then
        assert_eq!(result_1, None);
        assert_eq!(result_3, None);
        assert_eq!(result_5, None);
        let result_7 = result_7.unwrap();
        assert_eq!(result_7.channel(), ch(0));
        assert_eq!(result_7.number(), u14(420));
        assert_eq!(result_7.value(), u14(15000));
        assert!(result_7.is_registered());
        assert!(result_7.is_14_bit());
        assert_eq!(result_2, None);
        assert_eq!(result_4, None);
        let result_6 = result_6.unwrap();
        assert_eq!(result_6.channel(), ch(2));
        assert_eq!(result_6.number(), u14(421));
        assert_eq!(result_6.value(), u14(126));
        assert!(!result_6.is_registered());
        assert!(!result_6.is_14_bit());
    }

    #[test]
    fn should_ignore_non_contributing_short_messages_mixed() {
        // Given
        let mut scanner = ParameterNumberMessageScanner::new();
        // When
        let result_1 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(99), u7(3)));
        scanner.feed(&RawShortMessage::control_change(ch(2), cn(34), u7(5)));
        scanner.feed(&RawShortMessage::note_on(ch(2), key_number(100), u7(105)));
        let result_2 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(98), u7(37)));
        scanner.feed(&RawShortMessage::control_change(ch(2), cn(50), u7(6)));
        let result_3 = scanner.feed(&RawShortMessage::control_change(ch(2), cn(6), u7(126)));
        // Then
        assert_eq!(result_1, None);
        assert_eq!(result_2, None);
        let result_3 = result_3.unwrap();
        assert_eq!(result_3.channel(), ch(2));
        assert_eq!(result_3.number(), u14(421));
        assert_eq!(result_3.value(), u14(126));
        assert!(!result_3.is_registered());
        assert!(!result_3.is_14_bit());
    }
}
