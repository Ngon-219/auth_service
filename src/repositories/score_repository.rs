use crate::entities::{score_board, semester_summary, certificate};
use crate::static_service::DATABASE_CONNECTION;
use anyhow::Result;
use sea_orm::{
    ColumnTrait, DatabaseConnection, EntityTrait, QueryFilter, QueryOrder,
    ActiveModelTrait, Set,
};
use uuid::Uuid;
use chrono::Utc;
use sea_orm::prelude::Decimal;
use rand::Rng;

pub struct ScoreRepository;

impl ScoreRepository {
    pub fn new() -> Self {
        Self
    }

    pub fn get_connection(&self) -> &'static DatabaseConnection {
        DATABASE_CONNECTION
            .get()
            .expect("DATABASE_CONNECTION not set")
    }

    /// Get all scoreboard records for a user up to current date
    pub async fn get_scoreboard_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<score_board::Model>> {
        let db = self.get_connection();
        let now = Utc::now().naive_utc();
        
        let scores = score_board::Entity::find()
            .filter(score_board::Column::UserId.eq(user_id))
            .filter(score_board::Column::CreatedAt.lte(now))
            .order_by_asc(score_board::Column::AcademicYear)
            .order_by_asc(score_board::Column::Semester)
            .all(db)
            .await?;
        
        Ok(scores)
    }

    /// Get all semester summaries for a user up to current date
    pub async fn get_semester_summaries_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<semester_summary::Model>> {
        let db = self.get_connection();
        let now = Utc::now().naive_utc();
        
        let summaries = semester_summary::Entity::find()
            .filter(semester_summary::Column::UserId.eq(user_id))
            .filter(semester_summary::Column::CreatedAt.lte(now))
            .order_by_asc(semester_summary::Column::AcademicYear)
            .order_by_asc(semester_summary::Column::Semester)
            .all(db)
            .await?;
        
        Ok(summaries)
    }

    /// Get certificates by user_id and certificate type
    pub async fn get_certificates_by_user_id_and_type(
        &self,
        user_id: Uuid,
        certificate_type: &str,
    ) -> Result<Vec<certificate::Model>> {
        let db = self.get_connection();
        
        let certificates = certificate::Entity::find()
            .filter(certificate::Column::UserId.eq(user_id))
            .filter(certificate::Column::CertificateType.eq(certificate_type))
            .order_by_desc(certificate::Column::IssuedDate)
            .all(db)
            .await?;
        
        Ok(certificates)
    }

    /// Get all certificates for a user
    pub async fn get_certificates_by_user_id(
        &self,
        user_id: Uuid,
    ) -> Result<Vec<certificate::Model>> {
        let db = self.get_connection();
        
        let certificates = certificate::Entity::find()
            .filter(certificate::Column::UserId.eq(user_id))
            .order_by_desc(certificate::Column::IssuedDate)
            .all(db)
            .await?;
        
        Ok(certificates)
    }

    /// Create mock certificate data for a user
    pub async fn create_mock_certificate(
        &self,
        user_id: Uuid,
        certificate_type: &str,
    ) -> Result<certificate::Model> {
        let db = self.get_connection();
        let mut rng = rand::rng();
        
        let now = Utc::now().naive_utc().date();
        let issued_date = now - chrono::Duration::try_days(rng.random_range(0..365)).unwrap_or(chrono::Duration::zero());
        let expiry_date = Some(now + chrono::Duration::try_days(rng.random_range(365..1825)).unwrap_or(chrono::Duration::zero()));

        let certificate = certificate::ActiveModel {
            user_id: Set(user_id),
            certificate_type: Set(certificate_type.to_string()),
            issued_date: Set(issued_date),
            expiry_date: Set(expiry_date),
            description: Set(Some(format!("Mock {} certificate", certificate_type))),
            metadata: Set(Some(serde_json::json!({
                "mock": true,
                "created_at": now.to_string(),
            }))),
            ..Default::default()
        };

        let result = certificate.insert(db).await?;
        Ok(result)
    }

    /// Create mock diploma data for a user
    pub async fn create_mock_diploma(
        &self,
        user_id: Uuid,
    ) -> Result<certificate::Model> {
        self.create_mock_certificate(user_id, "Diploma").await
    }

    /// Create mock certificate with full data
    pub async fn create_certificate_with_data(
        &self,
        user_id: Uuid,
        certificate_type: &str,
        issued_date: chrono::NaiveDate,
        expiry_date: Option<chrono::NaiveDate>,
        description: Option<&str>,
        metadata: Option<serde_json::Value>,
    ) -> Result<certificate::Model> {
        let db = self.get_connection();

        let certificate = certificate::ActiveModel {
            user_id: Set(user_id),
            certificate_type: Set(certificate_type.to_string()),
            issued_date: Set(issued_date),
            expiry_date: Set(expiry_date),
            description: Set(description.map(|s| s.to_string())),
            metadata: Set(metadata),
            ..Default::default()
        };

        let result = certificate.insert(db).await?;
        Ok(result)
    }

    /// Create mock transcript data (scoreboard and semester summaries)
    pub async fn create_mock_transcript(
        &self,
        user_id: Uuid,
    ) -> Result<(Vec<score_board::Model>, Vec<semester_summary::Model>)> {
        let db = self.get_connection();
        let mut rng = rand::rng();
        
        // Generate data for 4-8 semesters
        let num_semesters = rng.random_range(4..9);
        let mut scoreboard_records = Vec::new();
        let mut semester_summaries = Vec::new();
        
        // Remove unused variable
        let academic_years = vec!["2020-2021", "2021-2022", "2022-2023", "2023-2024", "2024-2025"];
        let semesters = vec!["HK1", "HK2", "HK3"];
        let course_names = vec![
            "Toán học đại cương", "Vật lý đại cương", "Hóa học đại cương",
            "Lập trình C++", "Cấu trúc dữ liệu", "Giải tích", "Đại số tuyến tính",
            "Xác suất thống kê", "Cơ sở dữ liệu", "Mạng máy tính", "Hệ điều hành",
            "Công nghệ phần mềm", "Trí tuệ nhân tạo", "Đồ họa máy tính",
        ];
        let letter_grades = vec!["A", "B+", "B", "C+", "C", "D+", "D", "F"];
        // Status is determined by score, no need for separate vector

        for i in 0..num_semesters {
            let academic_year = academic_years[i % academic_years.len()];
            let semester = semesters[i % semesters.len()];
            
            // Generate 5-8 courses per semester
            let num_courses = rng.random_range(5..9);
            let mut total_credits = 0;
            let mut total_passed_credits = 0;
            let mut weighted_sum = Decimal::ZERO;

            for j in 0..num_courses {
                let course_name = course_names[rng.random_range(0..course_names.len())];
                let credits = rng.random_range(2..5);
                let score = rng.random_range(50..101) as f64;
                let letter_grade = letter_grades[rng.random_range(0..letter_grades.len())];
                let status = if score >= 50.0 { "Đạt" } else { "Không đạt" };

                let score_decimal = Decimal::try_from(score).unwrap_or(Decimal::ZERO);
                
                if status == "Đạt" {
                    total_passed_credits += credits;
                    // Calculate GPA weight (A=4.0, B+=3.5, B=3.0, C+=2.5, C=2.0, D+=1.5, D=1.0, F=0.0)
                    let gpa_weight = match letter_grade {
                        "A" => Decimal::from(40),
                        "B+" => Decimal::from(35),
                        "B" => Decimal::from(30),
                        "C+" => Decimal::from(25),
                        "C" => Decimal::from(20),
                        "D+" => Decimal::from(15),
                        "D" => Decimal::from(10),
                        _ => Decimal::ZERO,
                    } / Decimal::from(10);
                    weighted_sum += gpa_weight * Decimal::from(credits);
                }
                total_credits += credits;

                let scoreboard = score_board::ActiveModel {
                    user_id: Set(user_id),
                    course_id: Set(format!("COURSE-{}-{}", i, j)),
                    course_name: Set(course_name.to_string()),
                    course_code: Set(Some(format!("CS{:03}", rng.random_range(100..999)))),
                    credits: Set(credits),
                    score1: Set(Some(score_decimal)),
                    score2: Set(Some(Decimal::try_from(rng.random_range(40..101) as f64).unwrap_or(Decimal::ZERO))),
                    score3: Set(Some(Decimal::try_from(rng.random_range(40..101) as f64).unwrap_or(Decimal::ZERO))),
                    letter_grade: Set(Some(letter_grade.to_string())),
                    status: Set(Some(status.to_string())),
                    semester: Set(semester.to_string()),
                    academic_year: Set(Some(academic_year.to_string())),
                    metadata: Set(Some(serde_json::json!({
                        "mock": true,
                    }))),
                    ..Default::default()
                };

                let inserted = scoreboard.insert(db).await?;
                scoreboard_records.push(inserted);
            }

            // Calculate GPA
            let gpa = if total_credits > 0 {
                weighted_sum / Decimal::from(total_credits)
            } else {
                Decimal::ZERO
            };

            // Determine classification
            let classification = match gpa.to_string().parse::<f64>().unwrap_or(0.0) {
                g if g >= 3.6 => Some("Xuất sắc".to_string()),
                g if g >= 3.2 => Some("Giỏi".to_string()),
                g if g >= 2.5 => Some("Khá".to_string()),
                g if g >= 2.0 => Some("Trung bình".to_string()),
                _ => Some("Yếu".to_string()),
            };

            let semester_summary = semester_summary::ActiveModel {
                user_id: Set(user_id),
                semester: Set(semester.to_string()),
                academic_year: Set(academic_year.to_string()),
                gpa: Set(gpa),
                classification: Set(classification),
                total_credits: Set(Some(total_credits)),
                total_passed_credits: Set(Some(total_passed_credits)),
                metadata: Set(Some(serde_json::json!({
                    "mock": true,
                }))),
                ..Default::default()
            };

            let inserted = semester_summary.insert(db).await?;
            semester_summaries.push(inserted);
        }

        Ok((scoreboard_records, semester_summaries))
    }

    /// Create mock transcript data with exactly 4 semesters
    pub async fn create_mock_transcript_4_semesters(
        &self,
        user_id: Uuid,
    ) -> Result<(Vec<score_board::Model>, Vec<semester_summary::Model>)> {
        let db = self.get_connection();
        
        // Generate ALL random data BEFORE any await to avoid Send issues
        let academic_years = vec!["2020-2021", "2021-2022", "2022-2023", "2023-2024"];
        let semesters = vec!["HK1", "HK2", "HK1", "HK2"]; // 4 semesters: HK1, HK2, HK1, HK2
        let course_names = vec![
            "Toán học đại cương", "Vật lý đại cương", "Hóa học đại cương",
            "Lập trình C++", "Cấu trúc dữ liệu", "Giải tích", "Đại số tuyến tính",
            "Xác suất thống kê", "Cơ sở dữ liệu", "Mạng máy tính", "Hệ điều hành",
            "Công nghệ phần mềm", "Trí tuệ nhân tạo", "Đồ họa máy tính",
            "Lập trình Java", "Lập trình Python", "Hệ thống thông tin", "An toàn thông tin",
        ];

        // Pre-generate all data for all semesters before any await
        let mut all_course_data: Vec<Vec<(String, String, String, i32, Decimal, Decimal, Decimal, Decimal, Decimal, Decimal, String, String, String, String)>> = Vec::new();
        let mut all_semester_data: Vec<(String, String, Decimal, Option<String>, Option<i32>, Option<i32>)> = Vec::new();

        {
            let mut rng = rand::rng();
            for i in 0..4 {
                let academic_year = academic_years[i % academic_years.len()].to_string();
                let semester = semesters[i % semesters.len()].to_string();
                
                let num_courses = rng.random_range(5..9);
                let mut total_credits = 0;
                let mut total_passed_credits = 0;
                let mut weighted_sum = Decimal::ZERO;
                
                let mut course_data = Vec::new();
                for j in 0..num_courses {
                    let course_name = course_names[rng.random_range(0..course_names.len())].to_string();
                    let credits = rng.random_range(2..5);
                    let score = rng.random_range(50..101) as f64;
                    
                    let letter_grade = match score {
                        s if s >= 90.0 => "A",
                        s if s >= 85.0 => "B+",
                        s if s >= 80.0 => "B",
                        s if s >= 75.0 => "C+",
                        s if s >= 70.0 => "C",
                        s if s >= 65.0 => "D+",
                        s if s >= 60.0 => "D",
                        _ => "F",
                    };
                    
                    let status = if score >= 50.0 { "Đạt" } else { "Không đạt" };
                    let score_decimal = Decimal::try_from(score).unwrap_or(Decimal::ZERO);
                    
                    if status == "Đạt" {
                        total_passed_credits += credits;
                        let gpa_weight = match letter_grade {
                            "A" => Decimal::from(40),
                            "B+" => Decimal::from(35),
                            "B" => Decimal::from(30),
                            "C+" => Decimal::from(25),
                            "C" => Decimal::from(20),
                            "D+" => Decimal::from(15),
                            "D" => Decimal::from(10),
                            _ => Decimal::ZERO,
                        } / Decimal::from(10);
                        weighted_sum += gpa_weight * Decimal::from(credits);
                    }
                    total_credits += credits;

                    let score2 = Decimal::try_from(rng.random_range(40..101) as f64).unwrap_or(Decimal::ZERO);
                    let score3 = Decimal::try_from(rng.random_range(40..101) as f64).unwrap_or(Decimal::ZERO);
                    let score4 = Decimal::try_from(rng.random_range(40..101) as f64).unwrap_or(Decimal::ZERO);
                    let score5 = Decimal::try_from(rng.random_range(40..101) as f64).unwrap_or(Decimal::ZERO);
                    let score6 = Decimal::try_from(rng.random_range(40..101) as f64).unwrap_or(Decimal::ZERO);
                    let course_code = format!("CS{:03}", rng.random_range(100..999));

                    course_data.push((
                        format!("COURSE-{}-{}", i, j),
                        course_name,
                        course_code,
                        credits,
                        score_decimal,
                        score2,
                        score3,
                        score4,
                        score5,
                        score6,
                        letter_grade.to_string(),
                        status.to_string(),
                        semester.clone(),
                        academic_year.clone(),
                    ));
                }
                
                let gpa = if total_credits > 0 {
                    weighted_sum / Decimal::from(total_credits)
                } else {
                    Decimal::ZERO
                };

                let classification = match gpa.to_string().parse::<f64>().unwrap_or(0.0) {
                    g if g >= 3.6 => Some("Xuất sắc".to_string()),
                    g if g >= 3.2 => Some("Giỏi".to_string()),
                    g if g >= 2.5 => Some("Khá".to_string()),
                    g if g >= 2.0 => Some("Trung bình".to_string()),
                    _ => Some("Yếu".to_string()),
                };

                all_course_data.push(course_data);
                all_semester_data.push((
                    semester,
                    academic_year,
                    gpa,
                    classification,
                    Some(total_credits),
                    Some(total_passed_credits),
                ));
            }
        } // rng is dropped here, now we can await safely

        // Now insert all data (no rng in scope)
        let mut scoreboard_records = Vec::new();
        let mut semester_summaries = Vec::new();

        for course_data in all_course_data {
            for (course_id, course_name, course_code, credits, score1, score2, score3, score4, score5, score6, letter_grade, status, semester, academic_year) in course_data {
                let scoreboard = score_board::ActiveModel {
                    user_id: Set(user_id),
                    course_id: Set(course_id),
                    course_name: Set(course_name),
                    course_code: Set(Some(course_code)),
                    credits: Set(credits),
                    score1: Set(Some(score1)),
                    score2: Set(Some(score2)),
                    score3: Set(Some(score3)),
                    score4: Set(Some(score4)),
                    score5: Set(Some(score5)),
                    score6: Set(Some(score6)),
                    letter_grade: Set(Some(letter_grade)),
                    status: Set(Some(status)),
                    semester: Set(semester),
                    academic_year: Set(Some(academic_year)),
                    metadata: Set(Some(serde_json::json!({
                        "mock": true,
                    }))),
                    ..Default::default()
                };

                let inserted = scoreboard.insert(db).await?;
                scoreboard_records.push(inserted);
            }
        }

        for (semester, academic_year, gpa, classification, total_credits, total_passed_credits) in all_semester_data {
            let semester_summary = semester_summary::ActiveModel {
                user_id: Set(user_id),
                semester: Set(semester),
                academic_year: Set(academic_year),
                gpa: Set(gpa),
                classification: Set(classification),
                total_credits: Set(total_credits),
                total_passed_credits: Set(total_passed_credits),
                metadata: Set(Some(serde_json::json!({
                    "mock": true,
                }))),
                ..Default::default()
            };

            let inserted = semester_summary.insert(db).await?;
            semester_summaries.push(inserted);
        }

        Ok((scoreboard_records, semester_summaries))
    }
}

