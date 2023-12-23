use error_chain::error_chain;

error_chain! {
    errors{
        InvalidTargetFormat(s: String){
            description("Invalid Format for the from or to directives"),
            display("Invalid Target Format: {}", s)
        }
        InvalidSemantic(s: String){
            description("Invalid Semantic"),
            display("Invalid Semantic: {}", s)
        }
        InvalidName(s: String){
            description("Invalid Name"),
            display("Invalid Name: {}", s)
        }
        MemberNotFound(s: String){
            description("Member not found"),
            display("Member not found: {}", s)
        }
        GroupNotFound{
            description("The requested Group could not be found")
        }
        LogEntryNotFound {
            description("The requested Log Entry could not be found")
        }
    }

    foreign_links{
        IOError(std::io::Error);
        InvalidNumberFormat(std::num::ParseFloatError);
        YamlFormatError(serde_yaml::Error);
    }
}