import SwiftUI

/// O `@State` do SwiftUI, alcançável sem o Xcode.
///
/// A partir do SDK do macOS 26, `@State` deixou de ser uma property wrapper e
/// virou **macro** (`SwiftUIMacros.StateMacro`). O plugin que a implementa só
/// vem dentro do Xcode: no Command Line Tools ele não existe, e todo uso de
/// `@State` vira erro de compilação — o alvo do app parou de construir inteiro
/// quando o CLT subiu para 27.0 (60 erros, todos a mesma macro).
///
/// A macro é só um invólucro. A property wrapper de verdade continua no SDK,
/// como `SwiftUICore.State`, e é o que a macro acaba gerando. Nomeá-la por um
/// `typealias` evita a resolução de atributo cair na macro — escrever
/// `@SwiftUICore.State` direto ainda acha a macro primeiro e falha igual.
///
/// Semântica idêntica ao `@State`: mesmo tipo, mesmo `wrappedValue`, mesmo
/// `$projectedValue` devolvendo `Binding`. Quando o plugin passar a vir no CLT
/// (ou quando este projeto passar a buildar com Xcode), trocar de volta é um
/// `sed` — nada mais depende deste nome.
typealias ViewState<Value> = SwiftUICore.State<Value>
