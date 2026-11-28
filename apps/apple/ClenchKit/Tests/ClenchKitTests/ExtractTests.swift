import Foundation
import Testing
@testable import ClenchKit

@Suite struct ExtractTests {
    @Test func versionComesFromTheEngine() {
        #expect(!ClenchFFI.version().isEmpty)
    }

    @Test func acceptancePageReadsAsInTheOtherHosts() throws {
        let (data, url) = try Repo.fixture("01-iankduncan.html")
        let doc = try DocumentLoader.extract(data, contentType: "text/html", url: url)
        #expect(doc.title == "Smart People Who Give a Shit Use Tools")
        #expect(doc.wordCount == 1815)
        let session = try Session(document: doc, settingsJson: nil)
        #expect(session.wordCount() == 1815)
        let f = session.frame()
        #expect(f.before + f.pivot + f.after == "LLMs")
    }

    #if os(macOS)
    /// The word count must equal what the native debugging tool reports for
    /// the same fixture. Needs `cargo build -p clench-cli` first.
    @Test func wordCountMatchesClenchCli() throws {
        let name = "05-static-matklad.html"
        let (data, url) = try Repo.fixture(name)
        let doc = try DocumentLoader.extract(data, contentType: "text/html", url: url)

        let cli = Repo.root.appending(path: "target/debug/clench")
        try #require(
            FileManager.default.isExecutableFile(atPath: cli.path(percentEncoded: false)),
            "build the native tool first: cargo build -p clench-cli")
        var args = ["extract", Repo.root.appending(path: "fixtures/\(name)").path(percentEncoded: false), "--json"]
        if let url { args += ["--url", url.absoluteString] }
        let process = Process()
        process.executableURL = cli
        process.arguments = args
        let out = Pipe()
        process.standardOutput = out
        try process.run()
        let json = out.fileHandleForReading.readDataToEndOfFile()
        process.waitUntilExit()
        try #require(process.terminationStatus == 0)

        struct CliDocument: Decodable {
            let title: String
            let word_count: UInt64
        }
        let native = try JSONDecoder().decode(CliDocument.self, from: json)
        #expect(native.word_count > 0)
        #expect(doc.wordCount == native.word_count)
        #expect(doc.title == native.title)
    }
    #endif

    @Test func errorsAreTyped() {
        #expect(throws: ClenchError.NoContent) {
            try DocumentLoader.extract(Data("  \n ".utf8), contentType: "text/plain", url: nil)
        }
    }

    @Test func invalidSettingsAreRejected() throws {
        let doc = try DocumentLoader.extract(Data("Read the post yourself.".utf8), contentType: "text/plain", url: nil)
        #expect(throws: ClenchError.self) {
            try Session(document: doc, settingsJson: #"{"wmp": 400}"#)
        }
        let session = try Session(document: doc, settingsJson: #"{"wpm": 400}"#)
        #expect(session.wpm() == 400)
    }
}
