import Cocoa
import ApplicationServices
import Foundation

func emit(_ value: Any) {
    guard JSONSerialization.isValidJSONObject(value), let data = try? JSONSerialization.data(withJSONObject: value, options: [.sortedKeys]), let text = String(data: data, encoding: .utf8) else { return }
    print(text); fflush(stdout)
}
func supported(_ app: NSRunningApplication?) -> Bool {
    guard let app = app, let id = app.bundleIdentifier else { return false }
    return ["com.openai.chat", "com.openai.codex", "com.openai.chatgpt", "dev.openai.codex"].contains(id)
}
func attr(_ element: AXUIElement, _ key: String) -> CFTypeRef? {
    var value: CFTypeRef?; guard AXUIElementCopyAttributeValue(element, key as CFString, &value) == .success else { return nil }; return value
}
func string(_ e: AXUIElement, _ key: String) -> String { attr(e,key) as? String ?? "" }
func role(_ e: AXUIElement) -> String { string(e,kAXRoleAttribute) }
func name(_ e: AXUIElement) -> String { let title=string(e,kAXTitleAttribute);return title.isEmpty ? string(e,kAXDescriptionAttribute) : title }
func nodes(_ root: AXUIElement) -> [AXUIElement] {
    var queue=[root];var result=[AXUIElement]();var index=0
    while index<queue.count && index<12000 {
        let e=queue[index];index+=1;result.append(e)
        queue.append(contentsOf: (attr(e,kAXChildrenAttribute) as? [AXUIElement]) ?? [])
    }
    return result
}
func norm(_ s:String)->String { s.lowercased().unicodeScalars.filter { CharacterSet.alphanumerics.contains($0) }.map(String.init).joined() }
func parseSelection(_ label:String)->[String:String]? {
    let aliases=[("extra high","xhigh"),("매우 높음","xhigh"),("medium","medium"),("minimal","minimal"),("xhigh","xhigh"),("ultra","ultra"),("high","high"),("none","none"),("low","low"),("max","max"),("중간","medium"),("높음","high"),("낮음","low"),("최대","max")]
    for (suffix,effort) in aliases where label.lowercased().hasSuffix(suffix) {
        let model=String(label.dropLast(suffix.count)).trimmingCharacters(in:.whitespacesAndNewlines)
        if model.lowercased().hasPrefix("gpt"){return ["model":model,"effort":effort]}
    }
    return nil
}
enum BridgeError: Error, CustomStringConvertible {
    case message(String)
    var description:String {switch self {case let .message(s):return s}}
}
func target()->(NSRunningApplication,AXUIElement)? {
    guard let app=NSWorkspace.shared.frontmostApplication, supported(app) else {return nil}
    let root=AXUIElementCreateApplication(app.processIdentifier)
    if let window=attr(root,kAXFocusedWindowAttribute) {return (app,unsafeBitCast(window,to:AXUIElement.self))}
    return nil
}
func snapshot() throws -> [String:Any] {
    guard AXIsProcessTrusted() else {throw BridgeError.message("접근성 권한이 필요합니다. / Allow Accessibility access for GPT Model Switcher and its macos-bridge helper.")}
    guard let (app,root)=target() else {throw BridgeError.message("ChatGPT 앱을 활성화하십시오. / Activate ChatGPT.")}
    let application = AXUIElementCreateApplication(app.processIdentifier)
    guard let windows = attr(application,kAXWindowsAttribute) as? [AXUIElement], windows.count == 1 else {
        throw BridgeError.message("Cannot uniquely identify one target window / 대상 창을 하나로 확정하지 못했습니다.")
    }
    let all=nodes(root);let pickers=all.filter {role($0)==kAXButtonRole && parseSelection(name($0)) != nil}
    let permissions=all.filter {role($0)==kAXButtonRole && ["권한 변경","Change permissions"].contains(name($0))}
    guard permissions.count==1, !all.contains(where:{["클라우드 채팅","Cloud chat","원격 채팅","Remote chat"].contains(name($0))}) else {throw BridgeError.message("Cannot verify a local work conversation / 로컬 작업 대화임을 확인하지 못했습니다.")}
    guard pickers.count==1,let selection=parseSelection(name(pickers[0])) else {throw BridgeError.message("Cannot uniquely identify model and reasoning control / 모델 버튼을 식별할 수 없습니다.")}
    let busy=all.contains {role($0)==kAXButtonRole && ["중지","중단","작업 중지","stop","stop generating","stop streaming","stop turn","현재 작업 조정"].contains(name($0).lowercased())}
    guard let focused = attr(application,kAXFocusedUIElementAttribute) else {throw BridgeError.message("Cannot identify input focus")}
    let focus = unsafeBitCast(focused,to:AXUIElement.self)
    var focusBudget=12000
    func focusPath(_ e:AXUIElement,_ path:[Int],_ depth:Int)->[Int]? {
        focusBudget-=1;if focusBudget<0{return nil}
        if CFEqual(e,focus){return path};if depth>40{return nil}
        for (i,child) in ((attr(e,kAXChildrenAttribute) as? [AXUIElement]) ?? []).enumerated(){if let p=focusPath(child,path+[i],depth+1){return p}}
        return nil
    }
    guard let path=focusPath(root,[],0) else {throw BridgeError.message("Focused element is outside target window")}
    let focusToken:[String:Any] = ["path":path,"role":role(focus),"name":name(focus),"id":string(focus,kAXIdentifierAttribute)]
    let document=all.first {role($0)=="AXWebArea"}.map{name($0)} ?? ""
    return ["active":true,"available":!busy,"busy":busy,"message":busy ? "응답 생성 중입니다. / Generation in progress." : "전환 준비 완료 / Ready to switch.","selection":selection,"context":["pid":app.processIdentifier,"title":name(root),"document":document,"focus":focusToken]]
}
// Validate the token before AX reads; malformed input must never crash the helper.
func decodeContext(_ token:String, now:Double) throws -> [String:Any] {
    guard let data=token.data(using:.utf8),
      let expected=try JSONSerialization.jsonObject(with:data) as? [String:Any],
      let deadline=expected["deadline"] as? Double, deadline.isFinite, deadline >= now,
      let pid=expected["pid"] as? Int, pid > 0,
      let title=expected["title"] as? String, !title.isEmpty,
      let document=expected["document"] as? String, !document.isEmpty,
      let focus=expected["focus"] as? [String:Any],
      let path=focus["path"] as? [Int], path.allSatisfy({$0 >= 0}),
      let role=focus["role"] as? String, !role.isEmpty,
      focus["name"] is String, focus["id"] is String else {
        throw BridgeError.message("Invalid or expired target context")
    }
    return expected
}
func contextsMatch(_ actual:[String:Any], _ expected:[String:Any]) -> Bool {
    guard let actualPid=actual["pid"] as? NSNumber, let expectedPid=expected["pid"] as? NSNumber,
      actualPid == expectedPid,
      let title=actual["title"] as? String, title == expected["title"] as? String,
      let document=actual["document"] as? String, document == expected["document"] as? String,
      let focus=actual["focus"] as? [String:Any], let expectedFocus=expected["focus"] as? [String:Any]
      else {return false}
    return NSDictionary(dictionary:focus).isEqual(to:expectedFocus)
}
func guarded(_ token:String) throws -> (AXUIElement,[String:Any]) {
    let expected=try decodeContext(token,now:Date().timeIntervalSince1970)
    let current=try snapshot()
    guard let actual=current["context"] as? [String:Any], contextsMatch(actual,expected),
      let (_,root)=target() else {throw BridgeError.message("Conversation or window changed")}
    guard current["busy"] as? Bool != true else {throw BridgeError.message("Generation in progress")}
    return (root,current)
}
#if BRIDGE_TESTS
func runBridgeTests() throws {
    let focus:[String:Any] = ["path":[0,2],"role":"AXTextArea","name":"","id":"composer"]
    let base:[String:Any] = ["pid":Int32(123),"title":"Test","document":"Unique test","focus":focus,"deadline":101.0]
    func token(_ value:[String:Any]) throws -> String {String(data:try JSONSerialization.data(withJSONObject:value),encoding:.utf8)!}
    func require(_ ok:Bool,_ message:String) throws {if !ok {throw BridgeError.message(message)}}
    let decoded=try decodeContext(token(base),now:100)
    try require(contextsMatch(base,decoded),"valid JSON token must match a native snapshot")
    for key in ["pid","title","document","focus","deadline"] {
        var invalid=base;invalid.removeValue(forKey:key)
        var rejected=false;do {_ = try decodeContext(token(invalid),now:100)} catch {rejected=true}
        try require(rejected,"missing field accepted: " + key)
    }
    var expired=false;do {_ = try decodeContext(token(base),now:102)} catch {expired=true}
    try require(expired,"expired context accepted")
    for (key,value) in [("pid",124 as Any),("title","Other" as Any),("document","Other" as Any),("focus",["path":[0,3],"role":"AXTextArea","name":"","id":"composer"] as Any)] {
        var changed=base;changed[key]=value
        try require(!contextsMatch(changed,decoded),"changed context accepted: " + key)
    }
    try require(parseSelection("GPT-6.1 Terra medium")?["effort"] == "medium","English selection")
    try require(parseSelection("GPT-6.1 Sol 매우 높음")?["effort"] == "xhigh","Korean selection")
    try require(parseSelection("Other high") == nil,"unsupported selection accepted")
    emit(["passed":14,"scope":"synthetic token, focus and selection checks; no AX or model writes"])
}
#endif
let args=Array(CommandLine.arguments.dropFirst())
AXUIElementSetMessagingTimeout(AXUIElementCreateSystemWide(),0.6)
do {
 switch args.first {
 case "access": emit(["granted":AXIsProcessTrusted()])
 case "request-access":
    let options=[kAXTrustedCheckOptionPrompt.takeUnretainedValue() as String:true] as CFDictionary
    emit(["granted":AXIsProcessTrustedWithOptions(options)])
 case "open-access-settings":
    guard let url=URL(string:"x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"), NSWorkspace.shared.open(url) else {
        throw BridgeError.message("Cannot open Accessibility settings / 손쉬운 사용 권한 설정을 열지 못했습니다.")
    }
    emit(["ok":true])
 case "announce":
    guard args.count==2 else {throw BridgeError.message("Missing announcement")}
    let application=NSApplication.shared
    application.setActivationPolicy(.prohibited)
    NSAccessibility.post(element:application,notification:.announcementRequested,userInfo:[.announcement:args[1],.priority:NSAccessibilityPriorityLevel.medium.rawValue])
    RunLoop.current.run(until:Date().addingTimeInterval(0.15))
    emit(["ok":true])
 case "foreground": emit(["active":supported(NSWorkspace.shared.frontmostApplication)])
 case "watch":
    let parent=getppid()
    emit(["active":supported(NSWorkspace.shared.frontmostApplication)])
    _ = NSWorkspace.shared.notificationCenter.addObserver(forName:NSWorkspace.didActivateApplicationNotification,object:nil,queue:.main){_ in emit(["active":supported(NSWorkspace.shared.frontmostApplication)])}
    _ = Timer.scheduledTimer(withTimeInterval:1,repeats:true){_ in if kill(parent,0) != 0 {exit(0)}}
    RunLoop.main.run()
 case "release":
    let deadline=Date().addingTimeInterval(1)
    while !CGEventSource.flagsState(.combinedSessionState).intersection([.maskControl,.maskAlternate,.maskShift,.maskCommand]).isEmpty {
      if Date()>deadline {throw BridgeError.message("Release shortcut keys and retry")}
      Thread.sleep(forTimeInterval:0.02)
    }
    emit(["ok":true])
 case "inspect": emit(try snapshot())
 case "check":
    guard args.count==2 else {throw BridgeError.message("Missing target context")}
    _ = try guarded(args[1]);emit(["ok":true])
 #if BRIDGE_TESTS
 case "self-test": try runBridgeTests()
 #endif
 default: throw BridgeError.message("Unknown bridge command")
 }
} catch {emit(["error":String(describing:error)])}

