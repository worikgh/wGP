data <- read.table('Simulations.txt', header=TRUE)
names <- names(data)

objective <- data[,1]
best.estimate <- data[,names[length(names)]]
oldpar <- par(mfrow=c(2,2))
ratio <- 100*(objective-best.estimate)/objective

plot(x=data[,"X0"], y=best.estimate, cex=.2, ylab="Estimate", xlab="Actual", main="Best Model")
hist(ratio, main="Error Ratio", density=10, xlab="Percent Error", freq=FALSE)
hist(objective, main="Objective Data", density=10, breaks=30, xlab="Age")
hist(objective-best.estimate, main="Differences", density=10, freq=FALSE, breaks=30)
