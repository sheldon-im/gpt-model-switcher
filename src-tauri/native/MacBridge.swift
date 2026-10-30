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
    let application = AXUIElementCreateApplication(app.processIdentifier)
    guard let focused = attr(application,kAXFocusedUIElementAttribute) else {throw BridgeError.message("Cannot identify input focus")}
    let focus = unsafeBitCast(focused,to:AXUIElement.self)
    func focusPath(_ e:AXUIElement,_ path:[Int],_ depth:Int)->[Int]? {
        if CFEqual(e,focus){return path};if depth>40{return nil}
        for (i,child) in ((attr(e,kAXChildrenAttribute) as? [AXUIElement]) ?? []).enumerated(){if let p=focusPath(child,path+[i],depth+1){return p}}
        return nil
    }
    guard let path=focusPath(root,[],0) else {throw BridgeError.message("Focused element is outside target window")}
    let focusToken:[String:Any] = ["path":path,"role":role(focus),"name":name(focus),"id":string(focus,kAXIdentifierAttribute)]
    let document=all.first {role($0)=="AXWebArea"}.map{name($0)} ?? ""
    return ["active":true,"available":!busy,"busy":busy,"message":busy ? "응답 생성 중입니다. / Generation in progress." : "전환 준비 완료 / Ready to switch.","selection":selection,"context":["pid":app.processIdentifier,"title":name(root),"document":document,"focus":focusToken]]
}
func guarded(_ token:String) throws -> (AXUIElement,[String:Any]) {
    guard let data=token.data(using:.utf8),let expected=try JSONSerialization.jsonObject(with:data) as? [String:Any] else {throw BridgeError.message("Invalid target context")}
    if let deadline=expected["deadline"] as? Double, Date().timeIntervalSince1970 > deadline {throw BridgeError.message("Switch deadline exceeded")}
    let current=try snapshot();guard let actual=current["context"] as? [String:Any],
      String(describing:actual["pid"]!)==String(describing:expected["pid"]!),
      actual["title"] as? String==expected["title"] as? String,
      actual["document"] as? String==expected["document"] as? String,
      NSDictionary(dictionary:actual["focus"] as? [String:Any] ?? [:]).isEqual(to:expected["focus"] as? [String:Any] ?? [:]),
      let (_,root)=target() else {throw BridgeError.message("Conversation or window changed")}
    guard current["busy"] as? Bool != true else {throw BridgeError.message("Generation in progress")}
    return (root,current)
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
 default: throw BridgeError.message("Unknown bridge command")
 }
} catch {emit(["error":String(describing:error)])}

