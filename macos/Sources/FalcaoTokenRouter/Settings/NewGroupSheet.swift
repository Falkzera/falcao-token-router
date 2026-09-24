import SwiftUI
import CCUsageCore

struct NewGroupSheet: View {
    let onCreate: (String) -> Void
    @Environment(\.dismiss) private var dismiss
    @ViewState private var name = ""

    var body: some View {
        VStack(alignment: .leading, spacing: 14) {
            Text("groups.new.title").font(.headline)
            Text("groups.new.detail").font(.caption).foregroundStyle(.secondary)
            TextField("groups.name.placeholder", text: $name)
                .textFieldStyle(.roundedBorder)
                .onSubmit(create)
            HStack {
                Spacer()
                Button("groups.cancel") { dismiss() }
                Button("groups.new.create", action: create)
                    .buttonStyle(.borderedProminent)
                    .disabled(name.trimmingCharacters(in: .whitespaces).isEmpty)
            }
        }
        .padding(20).frame(width: 340)
    }

    private func create() {
        let trimmed = name.trimmingCharacters(in: .whitespaces)
        guard !trimmed.isEmpty else { return }
        onCreate(trimmed); dismiss()
    }
}
