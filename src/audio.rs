/// Windows default audio device management.
/// Uses PowerShell with IPolicyConfig COM to set default audio devices.
/// Lists devices via MMDeviceEnumerator COM.

use std::process::Command;

/// An audio endpoint device.
#[derive(Clone)]
pub struct AudioDevice {
    pub name: String,
    pub id: String,
    pub is_default: bool,
}

/// List active audio endpoints.
/// flow: "render" for speakers, "capture" for microphones.
pub fn list_devices(flow: &str) -> Vec<AudioDevice> {
    let data_flow = if flow == "render" { 0 } else { 1 };

    let ps_script = format!(r#"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

[ComImport, Guid("BCDE0395-E52F-467C-8E3D-C4579291692E")]
internal class MMDevEnum {{ }}

[Guid("A95664D2-9614-4F35-A746-DE8DB63617E6"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IMMDeviceEnumerator {{
    int EnumAudioEndpoints(int dataFlow, int stateMask, out IMMDeviceCollection devices);
    int GetDefaultAudioEndpoint(int dataFlow, int role, out IMMDevice device);
}}

[Guid("0BD7A1BE-7A1A-44DB-8397-CC5392387B5E"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IMMDeviceCollection {{
    int GetCount(out int count);
    int Item(int index, out IMMDevice device);
}}

[Guid("D666063F-1587-4E43-81F1-B948E807363F"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IMMDevice {{
    int Activate(ref Guid iid, int clsCtx, IntPtr ap, [MarshalAs(UnmanagedType.IUnknown)] out object iface);
    int OpenPropertyStore(int access, out IPropertyStore props);
    int GetId([MarshalAs(UnmanagedType.LPWStr)] out string id);
    int GetState(out int state);
}}

[Guid("886d8eeb-8cf2-4446-8d02-cdba1dbdcf99"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IPropertyStore {{
    int GetCount(out int count);
    int GetAt(int index, out PK key);
    int GetValue(ref PK key, out PV value);
}}

[StructLayout(LayoutKind.Sequential)] public struct PK {{ public Guid fmtid; public int pid; }}
[StructLayout(LayoutKind.Sequential)] public struct PV {{ public short vt; short r1,r2,r3; public IntPtr data; }}

public class AH {{
    public static void Run() {{
        var e = (IMMDeviceEnumerator)(new MMDevEnum());
        IMMDeviceCollection col; e.EnumAudioEndpoints({df}, 1, out col);
        int cnt; col.GetCount(out cnt);
        string defId = "";
        try {{ IMMDevice dd; e.GetDefaultAudioEndpoint({df}, 0, out dd); dd.GetId(out defId); }} catch {{}}
        var nk = new PK(); nk.fmtid = new Guid("a45c254e-df1c-4efd-8020-67d146a850e0"); nk.pid = 14;
        for (int i = 0; i < cnt; i++) {{
            IMMDevice d; col.Item(i, out d); string id; d.GetId(out id);
            IPropertyStore ps; d.OpenPropertyStore(0, out ps); PV v; ps.GetValue(ref nk, out v);
            string nm = Marshal.PtrToStringUni(v.data);
            string def = id == defId ? "1" : "0";
            Console.WriteLine(def + "|" + id + "|" + nm);
        }}
    }}
}}
'@
[AH]::Run()
"#, df = data_flow);

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();

    let output = match output {
        Ok(o) => o,
        Err(_) => return Vec::new(),
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    stdout
        .lines()
        .filter_map(|line| {
            let parts: Vec<&str> = line.splitn(3, '|').collect();
            if parts.len() == 3 {
                Some(AudioDevice {
                    is_default: parts[0] == "1",
                    id: parts[1].to_string(),
                    name: parts[2].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Set the default audio device by endpoint ID.
pub fn set_default_device(device_id: &str) -> bool {
    // Uses IPolicyConfig COM interface via PowerShell
    let ps_script = format!(r#"
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;

[Guid("F8679F50-850A-41CF-9C72-430F290290C8"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
internal interface IPolicyConfig {{
    int GetMixFormat(string id, IntPtr fmt);
    int GetDeviceFormat(string id, int def, IntPtr fmt);
    int ResetDeviceFormat(string id);
    int SetDeviceFormat(string id, IntPtr fmt, IntPtr fmtm);
    int GetProcessingPeriod(string id, int def, IntPtr defp, IntPtr minp);
    int SetProcessingPeriod(string id, IntPtr period);
    int GetShareMode(string id, IntPtr mode);
    int SetShareMode(string id, IntPtr mode);
    int GetPropertyValue(string id, int storeType, IntPtr key, IntPtr value);
    int SetPropertyValue(string id, int storeType, IntPtr key, IntPtr value);
    int SetDefaultEndpoint(string id, int role);
    int SetEndpointVisibility(string id, int visible);
}}

[ComImport, Guid("870AF99C-171D-4F9E-AF0D-E63DF40C2BC9")]
internal class PolicyConfigClient {{ }}

public class AudioSwitcher {{
    public static void SetDefault(string id) {{
        var policy = (IPolicyConfig)(new PolicyConfigClient());
        policy.SetDefaultEndpoint(id, 0); // eConsole
        policy.SetDefaultEndpoint(id, 1); // eMultimedia
        policy.SetDefaultEndpoint(id, 2); // eCommunications
    }}
}}
'@
[AudioSwitcher]::SetDefault("{id}")
"#, id = device_id.replace('"', ""));

    let output = Command::new("powershell")
        .args(["-NoProfile", "-Command", &ps_script])
        .output();

    match output {
        Ok(o) => o.status.success(),
        Err(_) => false,
    }
}
