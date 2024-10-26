from rest_framework import serializers
from .models import ApplicationRequest


class ApplicationRequestSerializer(serializers.ModelSerializer):
    created_by = serializers.StringRelatedField(default=serializers.CurrentUserDefault(), read_only=True)
    updated_by = serializers.StringRelatedField(default=serializers.CurrentUserDefault(), read_only=True)

    class Meta:
        model = ApplicationRequest
        fields = '__all__'
