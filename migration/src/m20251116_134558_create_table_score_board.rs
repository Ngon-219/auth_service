use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Create score_board table for individual course scores
        manager
            .create_table(
                Table::create()
                    .table(ScoreBoard::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(ScoreBoard::ScoreBoardId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(ScoreBoard::UserId).uuid().not_null())
                    .col(ColumnDef::new(ScoreBoard::CourseId).string().not_null())
                    .col(ColumnDef::new(ScoreBoard::CourseName).string().not_null())
                    .col(ColumnDef::new(ScoreBoard::CourseCode).string().null())
                    .col(
                        ColumnDef::new(ScoreBoard::Credits)
                            .integer()
                            .not_null()
                            .default(0),
                    )
                    .col(ColumnDef::new(ScoreBoard::Score1).decimal_len(5, 2).null())
                    .col(ColumnDef::new(ScoreBoard::Score2).decimal_len(5, 2).null())
                    .col(ColumnDef::new(ScoreBoard::Score3).decimal_len(5, 2).null())
                    .col(ColumnDef::new(ScoreBoard::Score4).decimal_len(5, 2).null())
                    .col(ColumnDef::new(ScoreBoard::Score5).decimal_len(5, 2).null())
                    .col(ColumnDef::new(ScoreBoard::Score6).decimal_len(5, 2).null())
                    .col(ColumnDef::new(ScoreBoard::LetterGrade).string().null())
                    .col(ColumnDef::new(ScoreBoard::Status).string().null())
                    .col(ColumnDef::new(ScoreBoard::Semester).string().not_null())
                    .col(ColumnDef::new(ScoreBoard::AcademicYear).string().null())
                    .col(ColumnDef::new(ScoreBoard::Metadata).custom("jsonb").null())
                    .col(
                        ColumnDef::new(ScoreBoard::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(ScoreBoard::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_score_board_user")
                            .from_tbl(ScoreBoard::Table)
                            .from_col(ScoreBoard::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create semester_summary table for semester GPA and classification
        manager
            .create_table(
                Table::create()
                    .table(SemesterSummary::Table)
                    .if_not_exists()
                    .col(
                        ColumnDef::new(SemesterSummary::SummaryId)
                            .uuid()
                            .not_null()
                            .primary_key()
                            .extra("DEFAULT gen_random_uuid()".to_string()),
                    )
                    .col(ColumnDef::new(SemesterSummary::UserId).uuid().not_null())
                    .col(ColumnDef::new(SemesterSummary::Semester).string().not_null())
                    .col(ColumnDef::new(SemesterSummary::AcademicYear).string().not_null())
                    .col(
                        ColumnDef::new(SemesterSummary::Gpa)
                            .decimal_len(5, 2)
                            .not_null()
                            .default(0.0),
                    )
                    .col(ColumnDef::new(SemesterSummary::Classification).string().null())
                    .col(ColumnDef::new(SemesterSummary::TotalCredits).integer().null())
                    .col(ColumnDef::new(SemesterSummary::TotalPassedCredits).integer().null())
                    .col(ColumnDef::new(SemesterSummary::Metadata).custom("jsonb").null())
                    .col(
                        ColumnDef::new(SemesterSummary::CreatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .col(
                        ColumnDef::new(SemesterSummary::UpdatedAt)
                            .timestamp()
                            .not_null()
                            .extra("DEFAULT CURRENT_TIMESTAMP".to_string()),
                    )
                    .foreign_key(
                        ForeignKey::create()
                            .name("fk_semester_summary_user")
                            .from_tbl(SemesterSummary::Table)
                            .from_col(SemesterSummary::UserId)
                            .to_tbl(User::Table)
                            .to_col(User::UserId)
                            .on_delete(ForeignKeyAction::Cascade)
                            .on_update(ForeignKeyAction::Cascade),
                    )
                    .to_owned(),
            )
            .await?;

        // Create indexes for score_board table
        manager
            .create_index(
                Index::create()
                    .name("idx_score_board_user_id")
                    .table(ScoreBoard::Table)
                    .col(ScoreBoard::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_score_board_user_semester")
                    .table(ScoreBoard::Table)
                    .col(ScoreBoard::UserId)
                    .col(ScoreBoard::Semester)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("idx_score_board_course_id")
                    .table(ScoreBoard::Table)
                    .col(ScoreBoard::CourseId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("unique_user_course_semester")
                    .table(ScoreBoard::Table)
                    .col(ScoreBoard::UserId)
                    .col(ScoreBoard::CourseId)
                    .col(ScoreBoard::Semester)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create indexes for semester_summary table
        manager
            .create_index(
                Index::create()
                    .name("idx_semester_summary_user_id")
                    .table(SemesterSummary::Table)
                    .col(SemesterSummary::UserId)
                    .to_owned(),
            )
            .await?;

        manager
            .create_index(
                Index::create()
                    .name("unique_user_semester_year")
                    .table(SemesterSummary::Table)
                    .col(SemesterSummary::UserId)
                    .col(SemesterSummary::Semester)
                    .col(SemesterSummary::AcademicYear)
                    .unique()
                    .to_owned(),
            )
            .await?;

        // Create function to calculate GPA from letter grades
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE OR REPLACE FUNCTION calculate_gpa_from_letter(letter_grade TEXT)
                RETURNS DECIMAL(5,2) AS $$
                BEGIN
                    RETURN CASE
                        WHEN letter_grade = 'A+' THEN 4.0
                        WHEN letter_grade = 'A' THEN 4.0
                        WHEN letter_grade = 'B+' THEN 3.5
                        WHEN letter_grade = 'B' THEN 3.0
                        WHEN letter_grade = 'C+' THEN 2.5
                        WHEN letter_grade = 'C' THEN 2.0
                        WHEN letter_grade = 'D+' THEN 1.5
                        WHEN letter_grade = 'D' THEN 1.0
                        WHEN letter_grade = 'F' THEN 0.0
                        ELSE NULL
                    END;
                END;
                $$ LANGUAGE plpgsql;
                "#,
            )
            .await?;

        // Create function to determine classification from GPA
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE OR REPLACE FUNCTION get_classification_from_gpa(gpa DECIMAL)
                RETURNS TEXT AS $$
                BEGIN
                    RETURN CASE
                        WHEN gpa >= 3.6 THEN 'Xuất sắc'
                        WHEN gpa >= 3.2 THEN 'Giỏi'
                        WHEN gpa >= 2.5 THEN 'Khá'
                        WHEN gpa >= 2.0 THEN 'Trung bình'
                        WHEN gpa >= 1.0 THEN 'Trung bình yếu'
                        ELSE 'Yếu'
                    END;
                END;
                $$ LANGUAGE plpgsql;
                "#,
            )
            .await?;

        // Create function to update semester summary
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE OR REPLACE FUNCTION update_semester_summary()
                RETURNS TRIGGER AS $$
                DECLARE
                    v_user_id UUID;
                    v_semester TEXT;
                    v_academic_year TEXT;
                    v_gpa DECIMAL(5,2);
                    v_total_credits INTEGER;
                    v_total_passed_credits INTEGER;
                    v_classification TEXT;
                BEGIN
                    -- Determine which record to process (NEW for INSERT/UPDATE, OLD for DELETE)
                    IF TG_OP = 'DELETE' THEN
                        v_user_id := OLD.user_id;
                        v_semester := OLD.semester;
                        v_academic_year := OLD.academic_year;
                    ELSE
                        v_user_id := NEW.user_id;
                        v_semester := NEW.semester;
                        v_academic_year := NEW.academic_year;
                    END IF;

                    -- Calculate GPA, total credits, and passed credits
                    SELECT 
                        COALESCE(
                            SUM(calculate_gpa_from_letter(sb.letter_grade) * sb.credits) / NULLIF(SUM(sb.credits), 0),
                            0.0
                        ),
                        COALESCE(SUM(sb.credits), 0),
                        COALESCE(SUM(CASE WHEN sb.status = 'Đạt' THEN sb.credits ELSE 0 END), 0)
                    INTO v_gpa, v_total_credits, v_total_passed_credits
                    FROM score_board sb
                    WHERE sb.user_id = v_user_id
                      AND sb.semester = v_semester
                      AND (v_academic_year IS NULL OR sb.academic_year = v_academic_year)
                      AND sb.letter_grade IS NOT NULL;

                    -- Get classification
                    v_classification := get_classification_from_gpa(v_gpa);

                    -- Insert or update semester summary
                    INSERT INTO semester_summary (
                        user_id,
                        semester,
                        academic_year,
                        gpa,
                        classification,
                        total_credits,
                        total_passed_credits,
                        updated_at
                    )
                    VALUES (
                        v_user_id,
                        v_semester,
                        v_academic_year,
                        v_gpa,
                        v_classification,
                        v_total_credits,
                        v_total_passed_credits,
                        CURRENT_TIMESTAMP
                    )
                    ON CONFLICT (user_id, semester, academic_year)
                    DO UPDATE SET
                        gpa = EXCLUDED.gpa,
                        classification = EXCLUDED.classification,
                        total_credits = EXCLUDED.total_credits,
                        total_passed_credits = EXCLUDED.total_passed_credits,
                        updated_at = CURRENT_TIMESTAMP;

                    IF TG_OP = 'DELETE' THEN
                        RETURN OLD;
                    ELSE
                        RETURN NEW;
                    END IF;
                END;
                $$ LANGUAGE plpgsql;
                "#,
            )
            .await?;

        // Create trigger on score_board table
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                CREATE TRIGGER trigger_update_semester_summary
                AFTER INSERT OR UPDATE OR DELETE ON score_board
                FOR EACH ROW
                EXECUTE FUNCTION update_semester_summary();
                "#,
            )
            .await?;

        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        // Drop indexes for semester_summary first
        manager
            .drop_index(
                Index::drop()
                    .name("unique_user_semester_year")
                    .table(SemesterSummary::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_semester_summary_user_id")
                    .table(SemesterSummary::Table)
                    .to_owned(),
            )
            .await?;

        // Drop indexes for score_board
        manager
            .drop_index(
                Index::drop()
                    .name("unique_user_course_semester")
                    .table(ScoreBoard::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_score_board_course_id")
                    .table(ScoreBoard::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_score_board_user_semester")
                    .table(ScoreBoard::Table)
                    .to_owned(),
            )
            .await?;

        manager
            .drop_index(
                Index::drop()
                    .name("idx_score_board_user_id")
                    .table(ScoreBoard::Table)
                    .to_owned(),
            )
            .await?;

        // Drop trigger
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP TRIGGER IF EXISTS trigger_update_semester_summary ON score_board;
                "#,
            )
            .await?;

        // Drop functions
        manager
            .get_connection()
            .execute_unprepared(
                r#"
                DROP FUNCTION IF EXISTS update_semester_summary();
                DROP FUNCTION IF EXISTS get_classification_from_gpa(DECIMAL);
                DROP FUNCTION IF EXISTS calculate_gpa_from_letter(TEXT);
                "#,
            )
            .await?;

        // Drop tables
        manager
            .drop_table(Table::drop().table(SemesterSummary::Table).to_owned())
            .await?;

        manager
            .drop_table(Table::drop().table(ScoreBoard::Table).to_owned())
            .await?;

        Ok(())
    }
}

#[derive(DeriveIden)]
enum ScoreBoard {
    Table,
    ScoreBoardId,
    UserId,
    CourseId,
    CourseName,
    CourseCode,
    Credits,
    Score1,
    Score2,
    Score3,
    Score4,
    Score5,
    Score6,
    LetterGrade,
    Status,
    Semester,
    AcademicYear,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum SemesterSummary {
    Table,
    SummaryId,
    UserId,
    Semester,
    AcademicYear,
    Gpa,
    Classification,
    TotalCredits,
    TotalPassedCredits,
    Metadata,
    CreatedAt,
    UpdatedAt,
}

#[derive(DeriveIden)]
enum User {
    Table,
    UserId,
}

