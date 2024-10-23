from django.db import models


class Level(models.Model):
    name = models.CharField(max_length=200)
    description = models.TextField()

    def __str__(self):
        return self.name


class Subject(models.Model):
    name = models.CharField(max_length=200)
    description = models.TextField()
    level = models.ForeignKey(Level, on_delete=models.CASCADE)

    def __str__(self):
        return self.name


class Category(models.Model):
    name = models.CharField(max_length=200)
    description = models.TextField()
    subject = models.ForeignKey(Subject, on_delete=models.CASCADE)

    def __str__(self):
        return self.name


class Topic(models.Model):
    name = models.CharField(max_length=200)
    description = models.TextField()
    category = models.ForeignKey(Category, on_delete=models.CASCADE)

    def __str__(self):
        return self.name


class AbstractQuestion(models.Model):
    topics = models.ManyToManyField(Topic)
    question_text = models.TextField()
    solution_description = models.TextField()
    marks = models.IntegerField()
    time_limit = models.DurationField()

    class Meta:
        abstract = True

    def __str__(self):
        return self.question_text


class MultiChoiceQuestion(AbstractQuestion):
    pass


class TextQuestion(AbstractQuestion):
    solution_text = models.TextField()


class NumericalQuestion(AbstractQuestion):
    solution_value = models.DecimalField(max_digits=30, decimal_places=8)


class Choice(models.Model):
    multi_choice_question = models.ForeignKey(MultiChoiceQuestion, on_delete=models.CASCADE)
    choice_text = models.CharField(max_length=200)
    is_correct = models.BooleanField(default=False)

    def __str__(self):
        return self.choice_text
