#!/usr/bin/perl -w
use strict;
use warnings;

##my $tree = 'Remainder Q Negate Remainder Add Q If Gt Q Q Log Multiply Q Log Q Lt Q If Q Add If Q Multiply Q Multiply Float 11.109434096871013 Float 9.050382118174413 Q Q Multiply Negate Q Add Q Invert Q Multiply Q Q';

##my $tree = 'Gt Lt Q Float -1.78143467392908 Q';
##my $tree = "Multiply Log Negate Q Q";
my $tree = "Add Gt Remainder x y Float -0.1 Lt Remainder x y Float 0.1";
my %nodes = ();
my %terminals = ();
# Arity of symbols


my @tree = split(/\s+/, $tree);
my %syms = (
    Add => '"+"',
    Gt => '<<b>&gt;</b>>',
    If => '"If"',
    Invert => '"1/"',
    Log => '"log"',
    Lt => '<<b>&lt;</b>>',
    Multiply => '<&times;>',
    Negate => '<&minus;>',
    Remainder => '"%"',
    );

my $out = 'digraph tree {
    size="8.3,11.7";
    node [shape=plaintext];
';
my $inp = '';
## Write preamble
foreach my $n (@tree){
    $n eq 'Float' and next;
    if($n =~ /^-?\d+\.\d+$/){
        $inp .= sprintf("%0.3f ", $n);
        next;
    }
    defined($nodes{$n}) or $nodes{$n} = 0;
    $nodes{$n}++;
    $inp .= "$n$nodes{$n} ";
    if(defined($syms{$n})){
        ## A known symbol
        $out .= "$n".$nodes{$n}." [label=$syms{$n}];\n";
    }else{
        ## A identifier/terminal
        $out .= "$n".$nodes{$n}." [label=\"$n\"];\n";
        $terminals{$n} = 1;
    }
}



my %Arity = (Invert => 1,
             Log => 1,
             Negate => 1,
             Add => 2,
             Gt => 2,
             Lt => 2,
             Multiply => 2,
             Remainder => 2,
             If => 3,
    );


my @tree1 = split(/\s+/, $inp);
sub clean {
    my $n = shift or die;
    $n =~ s/^([a-z]+)\d+$/$1/i;
    $n
}
my @stack = ();
do {
    my $n = shift(@tree1);
    my $_n = &clean($n);
    my $arity = $Arity{$_n};
    if(defined($arity)){
        if(@stack){
            my $s1 = pop(@stack);
            $out .= "$s1 -> $n;\n";
        }
        push(@stack, $n);
        if($arity > 1) {
            push(@stack, $n);
        }
        if($arity > 2) {
            push(@stack, $n);
        }
    }else{
        # Terminal
        my $s = pop(@stack);
        $out .= "$s -> $n\n";
    }
}while(@tree1);
$out .= '    
}
';
print "$out";
warn "$inp\n";
