// swift-tools-version: 5.9
// HSIP iOS SDK - Message Verification for Banks

import PackageDescription

let package = Package(
    name: "HSIP",
    platforms: [
        .iOS(.v13),
        .macOS(.v10_15)
    ],
    products: [
        .library(
            name: "HSIP",
            targets: ["HSIP"]),
    ],
    targets: [
        .target(
            name: "HSIP",
            dependencies: []),
        .testTarget(
            name: "HSIPTests",
            dependencies: ["HSIP"]),
    ]
)
