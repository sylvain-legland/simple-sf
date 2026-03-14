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
            resources: [
                .process("i18n/Localizable.xcstrings"),
                .copy("Resources/Avatars"),
                .copy("Resources/SFData"),
                .copy("Resources/Locales")
            ],
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency")
            ],
            linkerSettings: [
                .unsafeFlags([
                    "-LSFEngine/target/release",
                    "-lsf_engine",
                    "-framework", "Security",
                    "-framework", "SystemConfiguration",
                ]),
            ]
        )
    ]
)
