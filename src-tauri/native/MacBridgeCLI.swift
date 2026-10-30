import Cocoa
import ApplicationServices
@main
enum BridgeCLI {
 static func main() {
let args=Array(CommandLine.arguments.dropFirst())
do {
    switch args.first {
    case "watch":
        let parent=getppid()
        observeForeground {emit(["active":$0])}
        _ = Timer.scheduledTimer(withTimeInterval:1,repeats:true){_ in if kill(parent,0) != 0 {exit(0)}}
        RunLoop.main.run()
    case "release":
        let deadline=Date().addingTimeInterval(1)
        while !CGEventSource.flagsState(.combinedSessionState).intersection([.maskControl,.maskAlternate,.maskShift,.maskCommand]).isEmpty {
            if Date()>deadline {throw BridgeError.message("Release shortcut keys and retry")}
            Thread.sleep(forTimeInterval:0.02)
        }
        emit(["ok":true])
    #if BRIDGE_TESTS
    case "self-test": try runBridgeTests()
    #endif
    default:
        if args.first=="announce" {NSApplication.shared.setActivationPolicy(.prohibited)}
        emit(try bridgeRequest(args))
        if ["announce","request-access"].contains(args.first ?? "") {RunLoop.current.run(until:Date().addingTimeInterval(0.25))}
    }
} catch {emit(["error":String(describing:error)])}

 }
}
