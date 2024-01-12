#[derive(Debug, PartialEq)]
pub struct BootArgs {
    console: String,
    reboot: String,
    panic: u8, // 1 or 0
    pci: bool, // 1: on, 0: off
}
impl BootArgs {
    /// Parse a string into a BootArgs structure.
    /// 
    /// # Example
    /// ```
    /// use Rustcracker::model::boot_args::BootArgs;
    /// 
    /// let default_args = BootArgs::default();
    /// let parsed_string = BootArgs::parse("console=ttyS0 reboot=k panic=1 pci=off").expect("This should not appear");
    /// 
    /// assert_eq!(default_args, parsed_string); // Assert that they are equal.
    /// ```
    pub fn parse(value: impl Into<String>) -> Result<BootArgs, &'static str> {
        let temp: String = value.into();
        let args = temp.split_whitespace();
        let mut console = String::new();
        let mut reboot = String::new();
        let mut panic: u8 = 1;
        let mut pci: bool = false;
        for arg in args {
            match arg.split_once('=') {
                Some(("console", value)) => console = value.to_string(),
                Some(("reboot", value)) => reboot = value.to_string(),
                Some(("panic", value)) if value == "0" => panic = 0,
                Some(("panic", value)) if value == "1" => panic = 1,
                Some(("pci", value)) if value == "off" => pci = false,
                Some(("pci", value)) if value == "on" => pci = true,
                None | _ => return Err("Fail to parse boot args."),
            }
        }
        Ok(Self {console, reboot, panic, pci})
    }
}
impl Default for BootArgs {
    fn default() -> Self {
        Self {
            console: "ttyS0".to_string(),
            reboot: "k".to_string(),
            panic: 1,
            pci: false, // off
        }
    }
}
impl ToString for BootArgs {
    fn to_string(&self) -> String {
        let pci = 
            if self.pci {
                "on"
            } else {
                "off"
            };
        format!(
            "console={} reboot={} panic={} pci={}", 
            self.console, self.reboot, self.panic, pci
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_kernel_args_default() {
        let expected = BootArgs{
            console: "ttyS0".to_string(),
            reboot: "k".to_string(),
            panic: 1,
            pci: false,
        };
        let testing = BootArgs::default();
        assert_eq!(expected, testing);
    }

    #[test]
    fn test_kernel_args_from_string() {
        let expected = BootArgs {
            console: "ttyS0".to_string(),
            reboot: "k".to_string(),
            panic: 0,
            pci: true,
        };
        let testing = BootArgs::parse("console=ttyS0 reboot=k panic=0 pci=on".to_string()).unwrap();
        assert_eq!(expected, testing);
    }

    #[test]
    fn test_kernel_args_to_string() {
        let expected = "console=ttyS0 reboot=k panic=0 pci=on".to_string();
        let testing = BootArgs {
            console: "ttyS0".to_string(),
            reboot: "k".to_string(),
            panic: 0,
            pci: true,
        }.to_string();
        assert_eq!(expected, testing);
    }
}