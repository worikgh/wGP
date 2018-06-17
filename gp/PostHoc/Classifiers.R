## Read in the results of running `classify_test` and report on the
## quality of the classifiers

## Input looks like this.  The first number before "Class" is a timing
## indicator and can be ignored.  This example had two classes, but
## could just as easilly have more.  Two is minimum

## 0 Class: "Fraud"  Classification: "Fraud" 0.8367358279070258 "Legit" 0.645729084827568 
## 0 Class: "Fraud"  Classification: "Fraud" 0.8367358279070258 "Legit" -1.643664423859219 
## 0 Class: "Fraud"  Classification: "Legit" 1.4075388772916866 "Fraud" 0.4996335727324815 
## 0 Class: "Legit"  Classification: "Fraud" -0.5063010452983301 "Legit" -1.1034806931213894 
## 0 Class: "Legit"  Classification: "Fraud" -0.8367358279070258 "Legit" -0.8473335644259267 
## 0 Class: "Legit"  Classification: "Legit" -0.22653024029686036 "Fraud" -0.8367358279070258 

## Get the file to read
args<-commandArgs(TRUE)
input <- args[1]
if(is.na(input)){
    input <- "TestInput.txt"
}

data <- read.table(input)

## Get rid of first column (done here as in the future that field will
## be eliminated and when that happens only this line needs to be
## changed
data <- data[, -1]

## The first column is constand "Class:" so eliminate it
data <- data[, -1]

## Now second column is constant: "Classification:" so get rid of it
data <- data[, -2]

## There will be a odd number of columns now: The first column is the
## actual class of the example.

## From there the columns are in pairs.  First is a class name, next
## is the likelihood of belonging as calculated by classifier
## system. The next column is the likelihood.  There is a pair of
## columns for each class that can be tested for in decreasing
## likelihood order

stopifnot(ncol(data) %% 2 == 1 )

## Counters for correct classifications and incorrect
correct <- 0
failed <- 0

for (i in 1:nrow(data)){
    row = data[i,]
    if(row[,1] == row[,2]){
        correct <- correct + 1
    }else{
        failed <-  failed + 1
    }
}
paste("Correct: ",correct)
paste("Failed: ",failed)
paste(sep="", "Goodness: ",sprintf("%0.2f",100*correct/(correct+failed)), '%')
