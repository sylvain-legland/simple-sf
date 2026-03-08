// swift-tools-version: 5.9
import PackageDescription

let package = Package(
    name: "SimpleSF",
    platforms: [.macOS(.v14)],
    products: [
        .executable(name: "SimpleSF", targets: ["SimpleSF"])
    ],
    targets: [
        .executableTarget(
            name: "SimpleSF",
            path: "SimpleSF",
            exclude: ["Resources"],
            resources: [
                .process("i18n/Localizable.xcstrings")
            ],
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency")
            ]
        )
    ]
)
