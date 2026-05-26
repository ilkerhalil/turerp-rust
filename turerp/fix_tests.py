#!/usr/bin/env python3
import re
import os

TESTS_DIR = "tests"

def process_file(filepath):
    with open(filepath, "r") as f:
        content = f.read()
    original = content

    # 1. Fix local fn create_test_app_state() -> async fn create_test_app_state()
    if "fn create_test_app_state()" in content:
        content = re.sub(
            r"fn create_test_app_state\(\)",
            "async fn create_test_app_state()",
            content,
        )
        # Fix create_app_state_in_memory(&config).expect(...)
        content = re.sub(
            r"create_app_state_in_memory\(&config\)\.expect\(\"app state creation failed\"\)",
            "create_app_state_in_memory(&config).await.expect(\"app state creation failed\")",
            content,
        )

    # 2. Add .await to calls to create_test_app_state() in async test functions
    # But only if it's not already awaited
    # Pattern: let x = create_test_app_state();
    content = re.sub(
        r"let\s+(\w+)\s+=\s+create_test_app_state\(\);",
        r"let \1 = create_test_app_state().await;",
        content,
    )
    # Pattern: let mut x = create_test_app_state();
    content = re.sub(
        r"let\s+mut\s+(\w+)\s+=\s+create_test_app_state\(\);",
        r"let mut \1 = create_test_app_state().await;",
        content,
    )
    # Pattern: let _ = create_test_app_state();
    content = re.sub(
        r"let\s+_\s+=\s+create_test_app_state\(\);",
        r"let _ = create_test_app_state().await;",
        content,
    )
    # Pattern: let _app_state = create_test_app_state();
    content = re.sub(
        r"let\s+_(\w+)\s+=\s+create_test_app_state\(\);",
        r"let _\1 = create_test_app_state().await;",
        content,
    )
    # Fix double async
    content = re.sub(
        r"async async fn create_test_app_state\(\)",
        "async fn create_test_app_state()",
        content,
    )

    if content != original:
        with open(filepath, "w") as f:
            f.write(content)
        print(f"Updated: {filepath}")
    else:
        print(f"Skipped: {filepath}")


if __name__ == "__main__":
    for filename in os.listdir(TESTS_DIR):
        if filename.endswith(".rs"):
            process_file(os.path.join(TESTS_DIR, filename))
