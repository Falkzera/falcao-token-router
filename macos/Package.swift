// swift-tools-version: 6.0
import PackageDescription

let package = Package(
    name: "FalcaoTokenRouter",
    platforms: [.macOS("26.0")],
    // Cada pasta com código tem um `agent.md` (ver CLAUDE.md). O SwiftPM não sabe
    // o que fazer com ele e avisa "unhandled file" em todo build; a lista diz
    // que são para ignorar. O `exclude` não aceita glob: pasta nova, linha nova
    // — e o aviso de volta é o lembrete.
    targets: [
        .target(name: "CCUsageCore", exclude: [
            "agent.md", "Aggregation/agent.md", "Alerts/agent.md", "Engine/agent.md",
            "Models/agent.md", "Parsing/agent.md", "Presentation/agent.md",
            "Pricing/agent.md", "Settings/agent.md", "StatusLine/agent.md", "Usage/agent.md",
        ]),
        .executableTarget(name: "FalcaoTokenRouter", dependencies: ["CCUsageCore"], exclude: [
            "agent.md", "Alerts/agent.md", "Panel/agent.md", "Settings/agent.md",
        ]),
        // A CLI que o app empacota: `statusline` (o sensor de uso, sem rede),
        // `launch <grupo>` (sobe a sessão no grupo), `is-group` (usado pela função
        // de shell), `rotate`, `measure` (a sonda) e `doctor`. É a cola entre o
        // app e o terminal.
        .executableTarget(name: "router", dependencies: ["CCUsageCore"],
                          exclude: ["agent.md"]),
        .testTarget(name: "CCUsageCoreTests", dependencies: ["CCUsageCore"],
                    exclude: ["agent.md"]),
    ]
)
