import Foundation

/// Onde está o binário `router` que o app empacota.
///
/// No app instalado ele é irmão do executável principal, em `Contents/MacOS/`.
/// Em desenvolvimento (`swift run`, testes), fica ao lado no diretório de build.
/// Os dois casos são "irmão do executável atual", então é isso que se procura
/// primeiro; o caminho do bundle é a garantia.
enum RouterBinary {
    static var path: String? {
        let fm = FileManager.default

        if let exec = Bundle.main.executableURL {
            let sibling = exec.deletingLastPathComponent().appending(path: "router")
            if fm.isExecutableFile(atPath: sibling.path) { return sibling.path }
        }
        let bundled = Bundle.main.bundleURL
            .appending(path: "Contents/MacOS/router")
        if fm.isExecutableFile(atPath: bundled.path) { return bundled.path }
        return nil
    }
}
