package de.richy.voxels;

import java.util.Map;

public record BlockState(
	String typeName,
	Map<String, String> properties
) {}