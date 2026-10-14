import SwiftUI

/// SwiftUI's `State` property wrapper, under a name the `@State` macro doesn't shadow.
///
/// Since the macOS 27 SDK, `@State` is a macro whose plugin (SwiftUIMacros) ships only with
/// Xcode, so it fails to compile with just the Command Line Tools. `@ViewState` uses the
/// property wrapper directly and behaves the same. Once the project requires Xcode, replace
/// `@ViewState` with `@State` and delete this file.
typealias ViewState = SwiftUI.State
