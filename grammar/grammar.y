%start prog
%%

prog : prog class_def
     | class_def
     ;

class_def : "CLASS" "IDENTIFIER" "LPAREN" parent_class_opt "RPAREN" block;

parent_class_opt :
                | "IDENTIFIER"
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
          | func_def
          | for_statement
          ;

if_statement : "IF" expression block;

let_statement : "LET" "IDENTIFIER" "EQ" expression;

for_statement : "FOR" "LPAREN" expression "SEMICOLON" expression "SEMICOLON" expression block;

func_def    : "DEF" "IDENTIFIER" "LPAREN" parameter_list_opt "RPAREN" block ;

parameter_list_opt :
                   | parameter_list
                   ;

parameter_list : "IDENTIFIER"
               | parameter_list "COMMA" "IDENTIFIER"
               ;


expression : variable
           | binary_expression
           | method_invocation
           | field_access
           | class_instance_creation
           | literal
           ;

variable : "IDENTIFIER";

binary_expression : expression bin_op expression;

bin_op : "PLUS"
       | "MINUS"
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
