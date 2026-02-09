import com.vanniktech.maven.publish.SonatypeHost

plugins {
  `java-library`
  id("com.vanniktech.maven.publish") version "0.28.0"
}

repositories {
  mavenCentral()
}

tasks.register<Exec>("buildRust") {
  workingDir = file("..")
  commandLine("cargo", "build", "--release")
}

tasks.named<ProcessResources>("processResources") {
  if (System.getenv("CI") == null) {
    dependsOn("buildRust")
    from("../../target/release") {
      include("*.dll", "*.so", "*.dylib")
      into("native")
    }
  }
}

mavenPublishing {
  publishToMavenCentral(SonatypeHost.CENTRAL_PORTAL)

  signAllPublications()

  coordinates("de.richy", "voxels_rs", "0.1.8")

  pom {
    name.set("voxels_rs")
    description.set("Java bindings for the voxels_rs library")
    inceptionYear.set("2026")
    url.set("https://github.com/Jpx3/voxels_rs")

    licenses {
      license {
        name.set("The Apache License, Version 2.0")
        url.set("http://www.apache.org/licenses/LICENSE-2.0.txt")
        distribution.set("http://www.apache.org/licenses/LICENSE-2.0.txt")
      }
    }

    developers {
      developer {
        id.set("Jpx3")
        name.set("Richy")
        url.set("https://github.com/Jpx3")
      }
    }

    scm {
      url.set("https://github.com/Jpx3/voxels_rs")
      connection.set("scm:git:git://github.com/Jpx3/voxels_rs.git")
      developerConnection.set("scm:git:ssh://git@github.com/Jpx3/voxels_rs.git")
    }
  }
}

dependencies {
  testImplementation(libs.junit)
  api(libs.commons.math3)
  implementation(libs.guava)
}

java {
  toolchain {
    languageVersion = JavaLanguageVersion.of(21)
  }
}