// Compiled with the exact C# bindings embedded in native/windows/monitor.ps1.
// These tests do not invoke Windows APIs or change the physical display state.
public static class WindowsScreenIdentityTests {
  private static void Check(bool condition, string message) {
    if(!condition) throw new System.Exception(message);
  }
  public static void Main() {
    Check(System.Runtime.InteropServices.Marshal.SizeOf(typeof(ScreenBridgeNative.Input))==(System.IntPtr.Size==8 ? 40 : 28), "Wake INPUT ABI size");
    var wake=ScreenBridgeNative.WakeInput();
    Check(wake.type==0 && wake.union.mouse.flags==1 && wake.union.mouse.x==0 && wake.union.mouse.y==0 && wake.union.mouse.data==0, "Wake event is not a zero-distance move only");
    var single=new System.Collections.Generic.List<ScreenBridgeNative.Panel> {new ScreenBridgeNative.Panel {id="external"}};
    ScreenBridgeNative.ValidateSignalOutput(single,1,"external");
    foreach(uint count in new uint[] {0,2}) {
      bool blocked=false;
      try {ScreenBridgeNative.ValidateSignalOutput(single,count,"external");} catch(System.Exception) {blocked=true;}
      Check(blocked,"Global monitor sleep accepted a missing or cloned physical output");
    }
    foreach(string id in new string[] {"", "other"}) {
      bool blocked=false;
      try {ScreenBridgeNative.ValidateSignalOutput(single,1,id);} catch(System.Exception) {blocked=true;}
      Check(blocked,"Monitor sleep accepted an unrelated output");
    }
    single.Add(new ScreenBridgeNative.Panel {id="independent"});
    bool multiRejected=false;
    try {ScreenBridgeNative.ValidateSignalOutput(single,2,"external");} catch(System.Exception) {multiRejected=true;}
    Check(multiRejected,"Monitor sleep accepted multiple active screens");
    single.RemoveAt(1); single[0].@internal=true;
    bool internalRejected=false;
    try {ScreenBridgeNative.ValidateSignalOutput(single,1,"external");} catch(System.Exception) {internalRejected=true;}
    Check(internalRejected,"Monitor sleep accepted an internal screen");
    foreach(uint technology in new uint[] {6,11,13,0x80000000}) Check(!ScreenBridgeNative.ExternalSignalConnector(technology),"Internal connector accepted");
    foreach(uint technology in new uint[] {0,4,5,10}) Check(ScreenBridgeNative.ExternalSignalConnector(technology),"External connector rejected");
    Check(System.Runtime.InteropServices.Marshal.SizeOf(typeof(ScreenBridgeNative.DisplayDevice)) == 840, "DISPLAY_DEVICEW ABI size");
    Check(System.Runtime.InteropServices.Marshal.SizeOf(typeof(ScreenBridgeNative.DisplayPath)) == 72, "DISPLAYCONFIG_PATH_INFO ABI size");
    Check(System.Runtime.InteropServices.Marshal.SizeOf(typeof(ScreenBridgeNative.DisplayMode)) == 64, "DISPLAYCONFIG_MODE_INFO ABI size");
    Check(System.Runtime.InteropServices.Marshal.OffsetOf(typeof(ScreenBridgeNative.DisplayMode),"a").ToInt32() == 16, "Display mode union offset");
    string path = @"\\?\DISPLAY#LEN1234#5&abc&0&UID123#{e6f07b5f-ee97-4a90-b076-33f57bf4eaa7}";
    System.Func<string, System.Collections.Generic.IEnumerable<string>> interfaces = delegate(string name) {
      return new string[] {path};
    };
    System.Func<System.IntPtr, ScreenBridgeNative.MonitorInfo> info = delegate(System.IntPtr handle) {
      bool resumed = handle.ToInt64() == 900;
      return new ScreenBridgeNative.MonitorInfo {
        device = resumed ? @"\\.\DISPLAY3" : @"\\.\DISPLAY1",
        monitor = new ScreenBridgeNative.Rect {left=resumed ? 1920 : 0, top=0, right=resumed ? 4480 : 1920, bottom=1080}
      };
    };
    var before = ScreenBridgeNative.ReadPanel(new System.IntPtr(100),info,interfaces);
    var after = ScreenBridgeNative.ReadPanel(new System.IntPtr(900),info,interfaces);
    Check(before.id == after.id, "Same device changed ID after HMONITOR/display number changed");
    Check(before.bounds.x == 0 && after.bounds.x == 1920 && after.bounds.width == 2560, "Did not resolve current monitor bounds");
    Check(before.name != after.name, "Did not refresh current display name");
    Check(before.id.StartsWith("win-monitor-") && before.id.Length == 76, "ID is not config-safe");
    Check(before.id == ScreenBridgeNative.StableScreenId(new string[] {path.ToLowerInvariant(),path}), "Case or duplicate paths changed identity");
    string other = path.Replace("UID123", "UID456");
    Check(before.id != ScreenBridgeNative.StableScreenId(new string[] {other}), "Distinct same-model displays share an ID");
    Check(ScreenBridgeNative.StableScreenId(new string[] {path,other}) == ScreenBridgeNative.StableScreenId(new string[] {other,path}), "Clone target order changed identity");
    bool rejected = false;
    try { ScreenBridgeNative.StableScreenId(new string[0]); }
    catch(System.Exception) { rejected = true; }
    Check(rejected, "Missing stable identity silently fell back to an unstable handle");
    string ddcBefore=ScreenBridgeNative.StableDdcId(new string[] {path},0);
    string ddcAfter=ScreenBridgeNative.StableDdcId(new string[] {path.ToLowerInvariant()},0);
    string ddcOther=ScreenBridgeNative.StableDdcId(new string[] {other},0);
    Check(ddcBefore==ddcAfter,"DDC identity changed after display renumbering");
    Check(ddcBefore!=ddcOther,"Same-model DDC monitors share an identity");
    Check(ddcBefore!=ScreenBridgeNative.StableDdcId(new string[] {path},1),"Physical monitor index was lost");
    var aliases=new System.Collections.Generic.Dictionary<string,string>(System.StringComparer.OrdinalIgnoreCase);
    ScreenBridgeNative.RememberDdcAlias(aliases,@"\\.\DISPLAY1#0",ddcBefore);
    ScreenBridgeNative.RememberDdcAlias(aliases,@"\\.\DISPLAY2#0",ddcOther);
    // Switching the first monitor can exchange the live DISPLAY numbers. Old
    // settings must still resolve to the physical targets captured beforehand.
    ScreenBridgeNative.RememberDdcAlias(aliases,@"\\.\DISPLAY1#0",ddcOther);
    ScreenBridgeNative.RememberDdcAlias(aliases,@"\\.\DISPLAY2#0",ddcBefore);
    Check(ScreenBridgeNative.ResolveDdcId(aliases,@"\\.\display1#0")==ddcBefore,"First DDC alias followed a reordered display number");
    Check(ScreenBridgeNative.ResolveDdcId(aliases,@"\\.\DISPLAY2#0")==ddcOther,"Second DDC alias followed a reordered display number");
    Check(ScreenBridgeNative.ResolveDdcId(aliases,ddcOther)==ddcOther,"Stable saved DDC ID changed");
    int attempts=0, waits=0;
    ScreenBridgeNative.RetryDdcWrite(delegate {attempts++;},delegate(int ms) {waits++;});
    Check(attempts==1 && waits==0,"Successful DDC writes were repeated or delayed");
    attempts=0;
    ScreenBridgeNative.RetryDdcWrite(delegate {
      attempts++;
      if(attempts<3) throw new System.ComponentModel.Win32Exception(1167,"Transient display topology change");
    },delegate(int ms) {Check(ms==100 || ms==200,"Unexpected retry delay"); waits++;});
    Check(attempts==3 && waits==2,"Transient DDC failures were not retried with a bound");
    attempts=0; waits=0;
    bool exhausted=false;
    try {
      ScreenBridgeNative.RetryDdcWrite(delegate {attempts++;throw new System.ComponentModel.Win32Exception(5,"DDC rejected");},delegate(int ms) {waits++;});
    } catch(System.ComponentModel.Win32Exception error) {exhausted=error.NativeErrorCode==5;}
    Check(exhausted && attempts==3 && waits==2,"Repeated DDC failure lost the error or exceeded the retry limit");
    attempts=0; waits=0;
    bool invalid=false;
    try {
      ScreenBridgeNative.RetryDdcWrite(delegate {attempts++;throw new System.ArgumentException("Invalid target");},delegate(int ms) {waits++;});
    } catch(System.ArgumentException) {invalid=true;}
    Check(invalid && attempts==1 && waits==0,"Invalid DDC selection was retried");
    System.Console.WriteLine("Windows stable DDC identity, legacy alias renumbering, bounded native retries, signal selection and INPUT ABI passed; no native calls");
  }
}
