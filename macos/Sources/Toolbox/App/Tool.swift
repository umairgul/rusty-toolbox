import SwiftUI

enum ToolCategory: CaseIterable, Identifiable {
    case generators, converters, formatters, design, images

    var id: Self { self }

    var title: String {
        switch self {
        case .generators: "Generators"
        case .converters: "Encoders & Converters"
        case .formatters: "Formatters & Testers"
        case .design: "Color & Design"
        case .images: "Images"
        }
    }
}

/// Every tool, in sidebar order within each category. To add a tool: add a case here, give it
/// a name, description, category and symbol, then return its view from `ToolDetail`.
enum Tool: String, CaseIterable, Identifiable {
    // Generators
    case password, names, ids, qr, lorem, fakeData
    // Encoders & Converters
    case hashes, encode, timestamps, jwt, formats, numberBase, textCase, cron
    // Formatters & Testers
    case json, regex, textDiff, sql, markdown
    // Color & Design
    case colorPicker, contrast, palettes, gradients
    // Images
    case imageConverter, imageMetadata, iconGenerator, imageBase64

    var id: Self { self }

    var name: String {
        switch self {
        case .password: "Passwords"
        case .names: "Names"
        case .ids: "IDs"
        case .qr: "QR Codes"
        case .lorem: "Lorem Ipsum"
        case .fakeData: "Fake Data"
        case .hashes: "Hashes"
        case .encode: "Encode / Decode"
        case .timestamps: "Timestamps"
        case .jwt: "JWT Decoder"
        case .formats: "Data Formats"
        case .numberBase: "Number Bases"
        case .textCase: "Text Case"
        case .cron: "Cron"
        case .json: "JSON Formatter"
        case .regex: "Regex Tester"
        case .textDiff: "Text Diff"
        case .sql: "SQL Formatter"
        case .markdown: "Markdown Preview"
        case .colorPicker: "Color Picker"
        case .contrast: "Contrast Checker"
        case .palettes: "Palettes"
        case .gradients: "Gradients"
        case .imageConverter: "Image Converter"
        case .imageMetadata: "Metadata Remover"
        case .iconGenerator: "Icon Generator"
        case .imageBase64: "Base64 Images"
        }
    }

    var summary: String {
        switch self {
        case .password:
            "Random passwords and memorable passphrases, generated locally with a cryptographically secure RNG."
        case .names:
            "Random names for projects, usernames, test people and companies."
        case .ids:
            "UUIDs, ULIDs and Nano IDs, singly or in bulk."
        case .qr:
            "QR codes for text, links and Wi-Fi networks. Export as PNG or SVG."
        case .lorem:
            "Placeholder text by paragraph, sentence or word, as plain text or HTML."
        case .fakeData:
            "Realistic test records for seeding databases, as CSV, JSON or SQL. Emails and phone numbers are fictional."
        case .hashes:
            "MD5, SHA-1, SHA-2 and BLAKE3 hashes of text or files, and checksum verification."
        case .encode:
            "Base64, URL percent-encoding, HTML entities and hex."
        case .timestamps:
            "Convert Unix timestamps and dates, and see them in any time zone."
        case .jwt:
            "Decode JSON Web Tokens, check expiry and verify HMAC signatures. Tokens never leave this computer."
        case .formats:
            "Convert between JSON, YAML, TOML and CSV."
        case .numberBase:
            "Convert integers between decimal, hex, octal, binary and any base up to 36."
        case .textCase:
            "Convert text and identifiers between camelCase, snake_case, kebab-case and more. Each line is converted separately."
        case .cron:
            "Explain a cron expression in plain English and list its next run times."
        case .json:
            "Format, minify and validate JSON. Numbers, escapes and key order are kept exactly as written."
        case .regex:
            "Test regular expressions with live highlighting, capture groups and replace. Supports lookaround and backreferences."
        case .textDiff:
            "Compare two texts side by side, with changed words highlighted."
        case .sql:
            "Format SQL queries for reading, or compact them onto one line."
        case .markdown:
            "Write Markdown with a live preview, then copy or save it as HTML."
        case .colorPicker:
            "Pick a color anywhere on screen, or enter any CSS color, and get it in every format."
        case .contrast:
            "Check text and background colors against WCAG 2 accessibility levels, and get passing alternatives."
        case .palettes:
            "Build a 50–950 shade scale and color harmonies from one color. Click a swatch to copy it."
        case .gradients:
            "Blend colors in OKLab, OKLCH or sRGB, and copy the CSS or the in-between steps."
        case .imageConverter:
            "Convert images between PNG, JPEG, WebP, AVIF and more, resizing and compressing in bulk. Opens HEIC and SVG too."
        case .imageMetadata:
            "See what a photo reveals (camera, date, GPS location) and save a clean copy. JPEG, PNG and WebP are cleaned without any quality loss."
        case .iconGenerator:
            "Make website favicons, a macOS .icns, a Windows .ico and PNGs at every size from one image or SVG."
        case .imageBase64:
            "Turn images into data URIs for HTML, CSS and Markdown, or data URIs back into image files."
        }
    }

    var category: ToolCategory {
        switch self {
        case .password, .names, .ids, .qr, .lorem, .fakeData: .generators
        case .hashes, .encode, .timestamps, .jwt, .formats, .numberBase, .textCase, .cron: .converters
        case .json, .regex, .textDiff, .sql, .markdown: .formatters
        case .colorPicker, .contrast, .palettes, .gradients: .design
        case .imageConverter, .imageMetadata, .iconGenerator, .imageBase64: .images
        }
    }

    /// An SF Symbol for the sidebar.
    var symbol: String {
        switch self {
        case .password: "key"
        case .names: "textformat.abc"
        case .ids: "number"
        case .qr: "qrcode"
        case .lorem: "text.alignleft"
        case .fakeData: "tablecells"
        case .hashes: "number.square"
        case .encode: "arrow.left.arrow.right"
        case .timestamps: "clock"
        case .jwt: "checkmark.seal"
        case .formats: "doc.text"
        case .numberBase: "01.square"
        case .textCase: "textformat"
        case .cron: "calendar.badge.clock"
        case .json: "curlybraces"
        case .regex: "asterisk"
        case .textDiff: "plusminus"
        case .sql: "cylinder.split.1x2"
        case .markdown: "doc.richtext"
        case .colorPicker: "eyedropper"
        case .contrast: "circle.lefthalf.filled"
        case .palettes: "swatchpalette"
        case .gradients: "paintpalette"
        case .imageConverter: "photo.on.rectangle.angled"
        case .imageMetadata: "info.circle"
        case .iconGenerator: "app.badge"
        case .imageBase64: "photo.badge.arrow.down"
        }
    }

    static func tools(in category: ToolCategory) -> [Tool] {
        allCases.filter { $0.category == category }
    }
}
