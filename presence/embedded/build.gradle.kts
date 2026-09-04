plugins {
    id("com.android.library")
}

val localDiscordSdk = rootProject.layout.projectDirectory.file("app/libs/discord_partner_sdk.aar").asFile
val discordSdkPath = providers.environmentVariable("DISCORD_SOCIAL_SDK_AAR")
    .orElse(localDiscordSdk.absolutePath)

android {
    namespace = "dev.pyanexy.ympresence"
    compileSdk = 35
    ndkVersion = "28.2.13676358"

    defaultConfig {
        minSdk = 26

        externalNativeBuild {
            cmake {
                cppFlags += listOf("-std=c++17", "-fexceptions", "-frtti")
                arguments += listOf("-DANDROID_STL=c++_shared")
            }
        }

        ndk {
            abiFilters += setOf("arm64-v8a", "armeabi-v7a", "x86", "x86_64")
        }
    }

    buildFeatures {
        prefab = true
    }

    externalNativeBuild {
        cmake {
            path = file("src/main/cpp/CMakeLists.txt")
            version = "3.22.1"
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

dependencies {
    compileOnly(files(discordSdkPath))
}
