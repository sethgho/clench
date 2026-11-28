// swift-tools-version: 6.0
import PackageDescription

// ClenchCore.xcframework and Sources/ClenchFFI/ClenchFFI.swift are generated
// by scripts/build-apple.sh (run `bun run apple:xcframework` from the repo root).
let package = Package(
    name: "ClenchKit",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "ClenchKit", targets: ["ClenchKit"]),
    ],
    targets: [
        .binaryTarget(name: "ClenchCore", path: "Artifacts/ClenchCore.xcframework"),
        .target(name: "ClenchFFI", dependencies: ["ClenchCore"]),
        .target(name: "ClenchKit", dependencies: ["ClenchFFI"]),
        .testTarget(name: "ClenchKitTests", dependencies: ["ClenchKit"]),
    ]
)
