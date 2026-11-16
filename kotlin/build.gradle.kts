// 루담톡 봇 SDK (Kotlin / Java) — 만들기: gradlew build  →  build/libs/ludamtalk-1.0.0.jar
plugins {
    kotlin("jvm") version "2.4.0"
    `java-library`
    `maven-publish`
}

group = "com.ludamtalk"
version = "1.0.0"

repositories { mavenCentral() }

dependencies {
    api("com.squareup.okhttp3:okhttp:4.12.0")
    api("org.jetbrains.kotlinx:kotlinx-coroutines-core:1.10.2")
    api("org.json:json:20240303")
    testImplementation(kotlin("test"))
}

// Java 11 이상에서 돌아가요
kotlin { compilerOptions { jvmTarget.set(org.jetbrains.kotlin.gradle.dsl.JvmTarget.JVM_11) } }
java {
    sourceCompatibility = JavaVersion.VERSION_11
    targetCompatibility = JavaVersion.VERSION_11
    withSourcesJar()
}

publishing {
    publications {
        create<MavenPublication>("lib") {
            artifactId = "ludamtalk"
            from(components["java"])
            pom {
                name.set("ludamtalk")
                description.set("루담톡(LudamTalk) 온라인채팅 봇 SDK")
                url.set("https://ludamtalk.com/developers")
                licenses { license { name.set("MIT") } }
            }
        }
    }
}

tasks.test { useJUnitPlatform(); testLogging { showStandardStreams = true } }
