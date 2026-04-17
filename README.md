**THIS SOFTWARE IS EXPERIMENTAL AND NOT INTENDED FOR PRODUCTION USE**

# Restrict Windows Telemetry
This project applies opinionated baselines to Windows 10/11 machines, derived from the Microsoft article at https://learn.microsoft.com/en-us/windows/privacy/manage-connections-from-windows-operating-system-components-to-microsoft-services, and archived at https://archive.is/3rWQb.

Some of the policies listed may actually decrease the security on the system; consult src/lgpo.yaml for more detail descriptions on these.

## Building
Simply run:
```
cargo build --release
```
