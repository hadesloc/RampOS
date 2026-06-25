pub async fn run_test_migrations(pool: &sqlx::PgPool) {
    let source_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../migrations");
    let unique_suffix = format!(
        "{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock should be after UNIX_EPOCH")
            .as_nanos()
    );
    let temp_dir =
        std::env::temp_dir().join(format!("rampos-test-migrations-{unique_suffix}"));
    std::fs::create_dir_all(&temp_dir).expect("Failed to create temp migration directory");
    let mut deferred_enum_additions = Vec::new();

    for entry in std::fs::read_dir(&source_dir).expect("Failed to list migrations") {
        let path = entry.expect("Failed to read migration entry").path();
        if !path.is_file() || path.extension().and_then(|ext| ext.to_str()) != Some("sql") {
            continue;
        }

        let file_name = path.file_name().expect("Migration should have a file name");
        let mut sql = std::fs::read_to_string(&path).expect("Failed to read migration");
        if matches!(
            file_name.to_str(),
            Some("037_travel_rule.sql" | "039_rescreening_runs.sql")
        ) {
            let mut filtered = Vec::new();
            for line in sql.lines() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("ALTER TYPE compliance_event_type ADD VALUE") {
                    deferred_enum_additions.push(trimmed.to_string());
                } else {
                    filtered.push(line);
                }
            }
            sql = filtered.join("\n");
        }
        std::fs::write(temp_dir.join(file_name), sql).expect("Failed to stage migration");
    }

    let migrator = sqlx::migrate::Migrator::new(temp_dir.clone())
        .await
        .expect("Failed to build test migrator");
    migrator
        .run(pool)
        .await
        .expect("Failed to run test migrations");

    for statement in deferred_enum_additions {
        sqlx::query(&statement)
            .execute(pool)
            .await
            .expect("Failed to apply deferred enum addition");
    }

    let _ = std::fs::remove_dir_all(temp_dir);
}
