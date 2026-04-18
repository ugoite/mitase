package main

import "testing"

func TestGoRequirement(t *testing.T) {
	if GoFeatureImpl() != "ok" {
		t.Fatalf("expected ok")
	}
}
