import Foundation
import Observation

/// A tool that can take files opened from Finder or dropped on the Dock icon.
protocol OpensFiles: AnyObject {
    func open(_ urls: [URL])
}

/// Holds each tool's model for the life of the app, so switching tools keeps what you typed.
/// Models are created the first time their tool is opened.
@Observable
final class ToolStore {
    @ObservationIgnored lazy var password = PasswordModel()
    @ObservationIgnored lazy var names = NamesModel()
    @ObservationIgnored lazy var ids = IdsModel()
    @ObservationIgnored lazy var qr = QrModel()
    @ObservationIgnored lazy var lorem = LoremModel()
    @ObservationIgnored lazy var fakeData = FakeDataModel()
    @ObservationIgnored lazy var hashes = HashesModel()
    @ObservationIgnored lazy var encode = EncodeModel()
    @ObservationIgnored lazy var timestamps = TimestampsModel()
    @ObservationIgnored lazy var jwt = JwtModel()
    @ObservationIgnored lazy var formats = DataFormatsModel()
    @ObservationIgnored lazy var numberBase = NumberBaseModel()
    @ObservationIgnored lazy var textCase = TextCaseModel()
    @ObservationIgnored lazy var cron = CronModel()
    @ObservationIgnored lazy var json = JsonFormatterModel()
    @ObservationIgnored lazy var regex = RegexTesterModel()
    @ObservationIgnored lazy var textDiff = TextDiffModel()
    @ObservationIgnored lazy var sql = SqlFormatterModel()
    @ObservationIgnored lazy var markdown = MarkdownModel()
    @ObservationIgnored lazy var colorPicker = ColorPickerModel()
    @ObservationIgnored lazy var contrast = ContrastModel()
    @ObservationIgnored lazy var palettes = PalettesModel()
    @ObservationIgnored lazy var gradients = GradientsModel()
    @ObservationIgnored lazy var imageConverter = ImageConverterModel()
    @ObservationIgnored lazy var imageMetadata = ImageMetadataModel()
    @ObservationIgnored lazy var iconGenerator = IconGeneratorModel()
    @ObservationIgnored lazy var imageBase64 = ImageBase64Model()

    func model(for tool: Tool) -> AnyObject {
        switch tool {
        case .password: password
        case .names: names
        case .ids: ids
        case .qr: qr
        case .lorem: lorem
        case .fakeData: fakeData
        case .hashes: hashes
        case .encode: encode
        case .timestamps: timestamps
        case .jwt: jwt
        case .formats: formats
        case .numberBase: numberBase
        case .textCase: textCase
        case .cron: cron
        case .json: json
        case .regex: regex
        case .textDiff: textDiff
        case .sql: sql
        case .markdown: markdown
        case .colorPicker: colorPicker
        case .contrast: contrast
        case .palettes: palettes
        case .gradients: gradients
        case .imageConverter: imageConverter
        case .imageMetadata: imageMetadata
        case .iconGenerator: iconGenerator
        case .imageBase64: imageBase64
        }
    }
}
