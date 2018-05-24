#!/usr/bin/perl -w
use warnings;
use strict;

my %population = ();

sub bnd($){
    ## Use Births and Deaths file
    my $fin = shift or die "Pass input file";
    open(my $fh, $fin) or die $!;
    while(my $l = <$fh>){
        
        chomp $l;

        ## Generation Note: When this is used while a simulation under way
        ## there will be a ncomplete line at the end
        $l =~ s/^(\d+) // or last;
        my $gen = $1;

        if($l =~ /^(\d+) \+ (\d+) = (\d+)\/(\d+\.\d+):/ or
           $l =~ /^(\d+) \+ (\d+) = (\d+)\/(NaN):/ or
           $l =~ /^(\d+) \+ (\d+) = (\d+)\/(0):/ or
           $l =~ /^(\d+) \+ (\d+) = (\d+)\/(inf):/
            ) {
            ## A birth of two parents

            ## Motehr, Father, Child IDs
            my ($m, $f, $c, $e) = ($1, $2, $3, $4);
            $population{$c} = {id => $c, mother => $m, father => $f, evaluation => $e, born => $gen};
            &record_birth($population{$c});
            next;
        }elsif($l =~ /(\d+)\/(\d+\.\d+):/ or
               $l =~ /(\d+)\/(NaN):/ or
               $l =~ /(\d+)\/(0):/ or
               $l =~ /(\d+)\/(inf):/
            ){
            ## A virgin birth
            my ($c, $e) = ($1, $2);
            $population{$c} = {id => $c, mother => 0, father => 0, evaluation => $e, born => $gen};
            &record_birth($population{$c});
            next;
        }elsif($l =~ /^RIP (\d+)$/ or
               $l =~ /^Individual died natural cuses: (\d+)/) {
            ## The death of a individual
            &record_death($gen, $population{$1});
            delete($population{$1});
        }else{    
            print "WTF?! '$l'\n";
        }
    }
}

#&bnd('BnD.txt');
#&reportBND();

&gen('Gen.txt');

sub gen($){
    my $fin = shift or die "Pass input file";
    open(my $fh, $fin) or die $!;
    while(my $l = <$fh>){
        
        chomp $l;
        
    }
}

my $tot = 0; # Total individuals to exist
my $tot_na = 0; # Total to exist with NaN as evaluation
my %evals = (); # Record how many individuals at each evaluaion
my %span_eval = (); # Record each individuals span by evaluation

sub record_birth($){
    my $i = shift or die;
    $tot++;
    my $e = $i->{evaluation};

    ## Record each evaluation
    defined($evals{$e}) or $evals{$e} = 0;
    $evals{$e}++;
}

my $best_span = 0;
sub record_death($$){
    my $gen = shift;
    my $i = shift;
    my $span = $gen - $i->{born};
    defined($best_span) or $best_span = 0;
    if($span > $best_span){
        print $i->{id}." died $gen at $span\n";
        $best_span = $span;
    }
    $span_eval{$span} or $span_eval{$span}= [];
    push(@{$span_eval{$span}}, $i->{evaluation});
}

sub reportBND(){
    print "$tot individuals\n";
    print "Span\tCount\tMean Eval\n";
    my @keys = keys %span_eval;
    foreach my $span (sort {$a<=>$b} @keys){
        my $tot = scalar(@{$span_eval{$span}});
        my $sum = 0;
        my @es = map{s/NaN/0/; $_} @{$span_eval{$span}};
        foreach my $e (@es){
            $sum += $e;
        }
        ##map{$sum += $_} map{$_ =~ /[\d\.]+/ or $_ = 0; $_} @es ;
        my $mean = $sum/$tot;
        print "$span\t$tot\t$mean\n";
    }
}



