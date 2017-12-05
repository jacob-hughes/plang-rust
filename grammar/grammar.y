%start prog
%implicit_tokens WHITESPACE COMMENT
%%

prog : prog class_def
     | class_def
     ;

class_def : "CLASS" "IDENTIFIER" "LPAREN" parent_class_opt "RPAREN" "LBRACE" class_body "RBRACE";

parent_class_opt :
                | "IDENTIFIER"
                ;

class_body : class_body_statement
           | class_body class_body_statement;

class_body_statement : method_def
                     | let_statement "SEMI"
                     ;

method_def    : "DEF" "IDENTIFIER" "LPAREN" parameter_list_opt "RPAREN" block ;

parameter_list_opt :
                   | parameter_list
                   ;

parameter_list : expression
               | parameter_list "COMMA" expression
               ;

block : "LBRACE" block_statements_opt "RBRACE";

block_statements_opt :
                     | block_statements
                     ;

block_statements : statement
                 | block_statements "SEMI" statement
                 ;

statement : expression
          | if_statement
          | let_statement
          | for_statement
          ;

if_statement : "IF" expression block;

let_statement : "LET" "IDENTIFIER" "EQ" expression;

for_statement : "FOR" "LPAREN" expression "SEMICOLON" expression "SEMICOLON" expression block;


expression : variable
           | binary_expression
           | method_invocation
           | field_access
           | class_instance_creation
           | literal
           ;

variable : "IDENTIFIER";

binary_expression : expression binary_arithmetic expression;

binary_arithmetic : "PLUS"
                  | "MINUS"
                  ;

binary_expression : expression binary_comparison expression;

binary_comparison : "EQEQ"
                  | "LTEQ"
                  | "GTEQ"
                  | "LT"
                  | "GT"
                  ;

method_invocation : "IDENTIFIER" "DOT" "IDENTIFIER" "LPAREN" parameter_list_opt "RPAREN";


field_access : "THIS" "DOT" "IDENTIFIER"
             | "IDENTIFIER" "DOT" "IDENTIFIER"
             ;

class_instance_creation : "NEW" "IDENTIFIER" "LPAREN" parameter_list_opt "RPAREN";

literal : "INT_LITERAL"
        | "BOOL_LITERAL"
        | "STR_LITERAL"
        ;
