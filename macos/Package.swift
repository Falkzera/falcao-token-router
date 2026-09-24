// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "FalcaoTokenRouter",
    platforms: [.macOS("26.0")],
    targets: [
        .target(name: "CCUsageCore"),
        .executableTarget(name: "FalcaoTokenRouter", dependencies: ["CCUsageCore"]),
        // A CLI que o app empacota: `router statusline` (o sensor de uso, sem
        // rede), `router launch <grupo>` (sobe a sessão no grupo) e
        // `router is-group` (usado pela função de shell). É a cola entre o app e
        // o terminal.
        .executableTarget(name: "router", dependencies: ["CCUsageCore"]),
        .testTarget(name: "CCUsageCoreTests", dependencies: ["CCUsageCore"]),
    ]
)
