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
    return (app,root)
}
func snapshot() throws -> [String:Any] {
    guard AXIsProcessTrusted() else {throw BridgeError.message("접근성 권한이 필요합니다. / Allow Accessibility access for GPT Model Switcher and its macos-bridge helper.")}
    guard let (app,root)=target() else {throw BridgeError.message("ChatGPT 앱을 활성화하십시오. / Activate ChatGPT.")}
    let all=nodes(root);let pickers=all.filter {role($0)==kAXButtonRole && parseSelection(name($0)) != nil}
    guard pickers.count==1,let selection=parseSelection(name(pickers[0])) else {throw BridgeError.message("Cannot uniquely identify model and reasoning control / 모델 버튼을 식별할 수 없습니다.")}
    let busy=all.contains {role($0)==kAXButtonRole && ["중지","중단","작업 중지","stop","stop generating","stop streaming","stop turn","현재 작업 조정"].contains(name($0).lowercased())}
    let document=all.first {role($0)=="AXWebArea"}.map{name($0)} ?? ""
    return ["active":true,"available":!busy,"busy":busy,"message":busy ? "응답 생성 중입니다. / Generation in progress." : "전환 준비 완료 / Ready to switch.","selection":selection,"context":["pid":app.processIdentifier,"title":name(root),"document":document]]
}
func guarded(_ token:String) throws -> (AXUIElement,[String:Any]) {
    guard let data=token.data(using:.utf8),let expected=try JSONSerialization.jsonObject(with:data) as? [String:Any] else {throw BridgeError.message("Invalid target context")}
    if let deadline=expected["deadline"] as? Double, Date().timeIntervalSince1970 > deadline {throw BridgeError.message("Switch deadline exceeded")}
    let current=try snapshot();guard let actual=current["context"] as? [String:Any],
      String(describing:actual["pid"]!)==String(describing:expected["pid"]!),
      actual["title"] as? String==expected["title"] as? String,
      actual["document"] as? String==expected["document"] as? String,
      let (_,root)=target() else {throw BridgeError.message("Conversation or window changed")}
    guard current["busy"] as? Bool != true else {throw BridgeError.message("Generation in progress")}
    return (root,current)
}
func overlays(_ root:AXUIElement)->[AXUIElement] {
    nodes(root).filter {role($0)==kAXMenuRole || string($0,kAXSubroleAttribute)==kAXDialogSubrole || string($0,kAXRoleDescriptionAttribute)=="list box"}
}
func choices(_ root:AXUIElement)->[AXUIElement] {
    let all=nodes(root);let direct=all.filter {[kAXMenuItemRole,kAXRadioButtonRole].contains(role($0))}
    return direct+overlays(root).flatMap{nodes($0).filter{[kAXButtonRole,kAXMenuItemRole,kAXRadioButtonRole,kAXRowRole].contains(role($0))}}
}
func choice(_ root:AXUIElement,_ names:[String])->AXUIElement? {choices(root).first {e in names.contains {norm(name(e).components(separatedBy:"\n").first ?? "")==norm($0)}}}
func press(_ element:AXUIElement,_ token:String)throws { _ = try guarded(token);guard AXUIElementPerformAction(element,kAXPressAction as CFString) == .success else {throw BridgeError.message("Accessibility action unavailable")}}
func key(_ code:CGKeyCode,_ token:String)throws {
    _ = try guarded(token)
    for down in [true,false] {guard let event=CGEvent(keyboardEventSource:nil,virtualKey:code,keyDown:down) else {throw BridgeError.message("Cannot send key")};event.flags=[];event.post(tap:.cghidEventTap)}
    Thread.sleep(forTimeInterval:0.08)
}
func openPicker(_ token:String)throws->AXUIElement {
    var (root,_)=try guarded(token)
    if overlays(root).isEmpty {
        let pickers=nodes(root).filter{role($0)==kAXButtonRole && parseSelection(name($0)) != nil}
        guard pickers.count==1 else {throw BridgeError.message("Ambiguous model control")};try press(pickers[0],token)
    }
    for _ in 0..<20 {Thread.sleep(forTimeInterval:0.05);(root,_)=try guarded(token);if !choices(root).isEmpty{return root}}
    throw BridgeError.message("Model menu timeout")
}
func selectModel(_ value:String,_ token:String)throws {
    var root=try openPicker(token)
    if let c=choice(root,[value]){try press(c,token);return}
    guard let models=choice(root,["모델 선택","Choose model","Select model","Advanced","고급"]) else {throw BridgeError.message("Model chooser unavailable")}
    try press(models,token);Thread.sleep(forTimeInterval:0.12);(root,_)=try guarded(token)
    guard let c=choice(root,[value]) else {throw BridgeError.message("Model not available in current view: "+value)};try press(c,token)
}
func selectEffort(_ value:String,_ token:String)throws {
    let aliases:[String:[String]]=["low":["Light","Low","낮음"],"medium":["Medium","Standard","중간"],"high":["High","Extended","높음"],"xhigh":["Extra High","xhigh","매우 높음"],"minimal":["Minimal"],"none":["None","없음"],"max":["Max","최대"],"ultra":["Ultra"]]
    var root=try openPicker(token)
    if let c=choice(root,aliases[value] ?? [value]){try press(c,token);return}
    let levels=["none","minimal","low","medium","high","xhigh","max","ultra"]
    guard let desired=levels.firstIndex(of:value),let initial=(try snapshot())["selection"] as? [String:String] else {throw BridgeError.message("Unknown reasoning level")}
    for _ in 0..<8 {
        let (_,state)=try guarded(token);guard let selected=state["selection"] as? [String:String],let current=levels.firstIndex(of:selected["effort"] ?? "") else {throw BridgeError.message("Cannot read current reasoning")}
        if current==desired {try key(36,token);return}
        guard norm(selected["model"] ?? "")==norm(initial["model"] ?? "") else {throw BridgeError.message("Power changed model; restore required")}
        (root,_)=try guarded(token)
        guard let control=choice(root,["파워","Power","Reasoning effort","추론 수준"]) else {throw BridgeError.message("Reasoning control not available")}
        guard AXUIElementSetAttributeValue(control,kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success,
          let (app,_)=target(), let focused=attr(AXUIElementCreateApplication(app.processIdentifier),kAXFocusedUIElementAttribute), CFEqual(focused,control) else {throw BridgeError.message("Reasoning control focus verification failed")}
        try key(current<desired ? 124 : 123,token)
    }
    throw BridgeError.message("Requested reasoning is unavailable")
}
func finish(_ token:String)throws {
    var (root,_)=try guarded(token)
    for _ in 0..<3 {if overlays(root).isEmpty{break};try key(53,token);(root,_)=try guarded(token)}
    let composers=nodes(root).filter {e in [kAXTextAreaRole,kAXTextFieldRole].contains(role(e)) && ["chatgpt","message","메시지","질문"].contains{ name(e).lowercased().contains($0) }}
    guard composers.count==1, AXUIElementSetAttributeValue(composers[0],kAXFocusedAttribute as CFString,kCFBooleanTrue) == .success else {throw BridgeError.message("Cannot restore composer focus")}
}

let args=Array(CommandLine.arguments.dropFirst())
AXUIElementSetMessagingTimeout(AXUIElementCreateSystemWide(),0.6)
do {
 switch args.first {
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
 case "check": _ = try guarded(args[1]);emit(["ok":true])
 case "model": try selectModel(args[1],args[2]);emit(["ok":true])
 case "effort": try selectEffort(args[1],args[2]);emit(["ok":true])
 case "finish": try finish(args[1]);emit(["ok":true])
 default: throw BridgeError.message("Unknown bridge command")
 }
} catch {emit(["error":String(describing:error)])}

