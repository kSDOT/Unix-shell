# Unix-shell
Basic unix shell with history feature written in rust

This is a shell interpreter I have written for a school project. It support most commands, with the addition of:  
"cd" -> changes current working directory  
"history[ n]" -> prints n lines of history, or 1 line if n is missing or bigger than the number of commands stored.  
"help" -> prints custom help message  
"exit"|"q" -> exits  

The parsing grammar is as follows:  
[Command[ (-Options|--Option)* ][ Args* ][PIPE|(STREAM[ ]OPERATOR[ ](FILE|HANDLE)][END]]*

where:  
  
Command  →   command to be executed  
Options  →   options for the command  
Args     →   arguments for the command  
PIPE     →   ‘|’ character, enables piping one commands output as the following commands input  
STREAM   →   number that represents a stream; accepted values: 0 -> stdin, 1 -> stdout, 2 -> stderr  
OPERATOR →   ‘<’ or ‘>’ character; indicates stream redirection direction:  
                    Command[..] < file_name   contents of ‘file_name’ serve as input for the command;  
                    Command[..] > file_name   contents of command are outputed to 'file_name';  
                    Command[..] > &n          n is a handle to a file: 0-> handle for input file,   
                                                                       1-> handle for output file,  
                                                                       2-> handle for error file;  
                    Command[..] n > x         stream indicated by 'n' is directed to/from x,  
                                              where x is handle or file_name  
FILE    →   path to a file  
HANDLE  →   &n where n represents a stream (0→input, 1→output, 2→error )  
END     →   ‘;’ character, indicates the end of a single command  
[]      →   optional  
\*     →   zero, one or some repetitions   

Spaces and hyphens must be used to separate words as indicated in the grammar expression.

Examples of accepted expressions:  
ls  
ls -a  
ls -aC --context  
ls -aC --context Desktop/folder_name  
ls -aC --context | cat  
cat < Desktop/file.txt  
cat 0>Desktop/file.txt  
cat 0>&1  
cat < Desktop/file.txt > Desktop/file1.txt  
ls | cat > Desktop/file.txt; ls -a  

NOTE: history[ n] command outputs the first N commands used. Also adding commands to history is done before executing commands, so that even if you execute history[ n] with an empty file, "history[ n]" will show up, as by the time the command is executed the history is already updated. Also, if n is greater than the number of command available, only 1 command will show up.
