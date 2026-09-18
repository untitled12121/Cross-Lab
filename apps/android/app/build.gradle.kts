import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    alias(libs.plugins.android.application)
    alias(libs.plugins.compose.compiler)
}

val repoRoot = rootProject.layout.projectDirectory.asFile.parentFile.parentFile
val generatedUniFfiKotlin = layout.buildDirectory.dir("generated/uniffi/kotlin")
val generatedJniLibs = layout.buildDirectory.dir("generated/jniLibs")

android {
    namespace = "dev.crosslab.android"
    compileSdk = 37
    ndkVersion = "28.2.13676358"

    defaultConfig {
        applicationId = "dev.crosslab.android"
        minSdk = 23
        targetSdk = 36
        versionCode = 1
        versionName = "0.1.0-dev"

        ndk {
            abiFilters += "arm64-v8a"
        }
    }

    buildFeatures {
        compose = true
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    sourceSets {
        getByName("main") {
            java.directories.add(generatedUniFfiKotlin.get().asFile.absolutePath)
            jniLibs.directories.add(generatedJniLibs.get().asFile.absolutePath)
            assets.directories.add(repoRoot.resolve("design/themes").absolutePath)
        }
        getByName("test") {
            resources.directories.add(repoRoot.resolve("design").absolutePath)
        }
    }

    packaging {
        jniLibs {
            useLegacyPackaging = false
        }
    }

    testOptions {
        unitTests.isIncludeAndroidResources = true
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

val generateUniFfiKotlin by tasks.registering(Exec::class) {
    workingDir(repoRoot)
    inputs.file(repoRoot.resolve("Cargo.lock"))
    inputs.file(repoRoot.resolve("Cargo.toml"))
    inputs.file(repoRoot.resolve("crates/mobile-ffi/Cargo.toml"))
    inputs.dir(repoRoot.resolve("crates/mobile-ffi/src"))
    inputs.file(repoRoot.resolve("scripts/generate-mobile-bindings.sh"))
    outputs.dir(generatedUniFfiKotlin)

    commandLine(
        "bash",
        repoRoot.resolve("scripts/generate-mobile-bindings.sh").absolutePath,
        generatedUniFfiKotlin.get().asFile.absolutePath,
    )
}

val buildRustAndroidArm64 by tasks.registering(Exec::class) {
    workingDir(repoRoot)
    inputs.file(repoRoot.resolve("Cargo.lock"))
    inputs.file(repoRoot.resolve("Cargo.toml"))
    inputs.file(repoRoot.resolve("crates/mobile-ffi/Cargo.toml"))
    inputs.dir(repoRoot.resolve("crates/mobile-ffi/src"))
    inputs.file(repoRoot.resolve("scripts/build-android-rust.sh"))
    outputs.dir(generatedJniLibs.map { it.dir("arm64-v8a") })

    commandLine(
        "bash",
        repoRoot.resolve("scripts/build-android-rust.sh").absolutePath,
        generatedJniLibs.get().asFile.absolutePath,
        "arm64-v8a",
    )
}

generateUniFfiKotlin {
    mustRunAfter(buildRustAndroidArm64)
}

tasks.named("preBuild") {
    dependsOn(generateUniFfiKotlin, buildRustAndroidArm64)
}

dependencies {
    implementation(platform(libs.androidx.compose.bom))
    implementation(libs.androidx.activity.compose)
    implementation(libs.androidx.lifecycle.process)
    implementation(libs.androidx.compose.foundation)
    implementation(libs.androidx.compose.ui)
    implementation(libs.androidx.compose.ui.tooling.preview)
    implementation("net.java.dev.jna:jna:5.19.1@aar")

    debugImplementation(libs.androidx.compose.ui.tooling)

    testImplementation(libs.junit)
    testImplementation(libs.json)
}
