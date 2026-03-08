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
                .process("Resources")
            ],
            swiftSettings: [
                .enableExperimentalFeature("StrictConcurrency")
            ]
        )
    ]
)
