plugins {
    id("com.android.application")
}

val defaultPayload = rootProject.layout.projectDirectory
    .dir("../dist")
    .file("yandex-music-stable-2026.08.4-162.1gpr-ympatcher-v0.5.0.apks")

val payloadApks = providers.environmentVariable("YMPATCHER_INSTALLER_APKS")
    .map { file(it) }
    .orElse(defaultPayload.asFile)

android {
    namespace = "dev.pyanexy.ympatcher.installer"
    compileSdk = 35

    defaultConfig {
        applicationId = "dev.pyanexy.ympatcher.installer"
        minSdk = 26
        targetSdk = 35
        versionCode = 5
        versionName = "0.5.0"
    }

    buildTypes {
        release {
            signingConfig = signingConfigs.getByName("debug")
            isMinifyEnabled = false
        }
    }
}

tasks.register<Copy>("copyApksPayload") {
    val source = payloadApks.get()
    require(source.isFile) { "Missing APKS payload: ${source.absolutePath}" }
    from(source)
    into(layout.projectDirectory.dir("src/main/assets"))
    rename { "payload.apks" }
}

tasks.named("preBuild") {
    dependsOn("copyApksPayload")
}
