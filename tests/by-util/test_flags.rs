#[cfg(test)]
mod tests {
    use nix::sys::termios::InputFlags;
    use nix::sys::termios::OutputFlags;
    use nix::sys::termios::ControlFlags;
    use nix::sys::termios::LocalFlags;

    #[test]
    fn test_flags_retain_unknown_bits() {
        // 0x8000_0000 is a high bit usually not defined in standard termios flags.
        // We use it to simulate an "unknown" kernel flag.
        let unknown_bit: u32 = 0x8000_0000;

        // Create the InputFlags from this raw bit
        let input = InputFlags::from_bits_retain(unknown_bit);
        let output = OutputFlags::from_bits_retain(unknown_bit);
        let control = ControlFlags::from_bits_retain(unknown_bit);
        let local = LocalFlags::from_bits_retain(unknown_bit);

        // Assert that the underlying bits are exactly what we put in.
        // If 'truncate' was used, these would be 0 (or strictly defined flags).
        assert_eq!(input.bits(), unknown_bit, "InputFlags did not retain unknown bits");
        assert_eq!(output.bits(), unknown_bit, "OutputFlags did not retain unknown bits");
        assert_eq!(control.bits(), unknown_bit, "ControlFlags did not retain unknown bits");
        assert_eq!(local.bits(), unknown_bit, "LocalFlags did not retain unknown bits");
    }
}