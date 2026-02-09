use std::fs;
use std::process::Command;
fn main() {
	// let java_src_dir = "java_src";
	// let out_dir = "../target/java_classes";
	// fs::create_dir_all(out_dir).unwrap();
	// let status = Command::new("javac")
	// 	.args([
	// 		"-d", out_dir,
	// 		"-sourcepath", java_src_dir,
	// 		&format!("{}/de/richy/voxels/Voxels.java", java_src_dir)
	// 	])
	// 	.status()
	// 	.expect("Failed to run javac");
    //
	// if !status.success() {
	// 	panic!("Java compilation failed!");
	// }
    //
	// let jar_status = Command::new("jar")
	// 	.args([
	// 		"cf",
	// 		"../target/voxels.jar",
	// 		"-C",
	// 		out_dir,
	// 		"."
	// 	])
	// 	.status()
	// 	.expect("Failed to run jar command");
    //
	// if !jar_status.success() {
	// 	panic!("Creating JAR file failed!");
	// }
}