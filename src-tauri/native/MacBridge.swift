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
    var description:String { if case let .message(s)=self {return s} }
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
let args=Array(CommandLine.arguments.dropFirst())
do {
 switch args.first {
 case "foreground": emit(["active":supported(NSWorkspace.shared.frontmostApplication)])
 case "inspect": emit(try snapshot())
 default: throw BridgeError.message("Unknown bridge command")
 }
} catch {emit(["error":String(describing:error)])}

